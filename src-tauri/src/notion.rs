use crate::models::{Settings, Task};
use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde_json::{json, Map, Value};

const NOTION_VERSION: &str = "2026-03-11";
const NOTION_API: &str = "https://api.notion.com/v1";

pub struct NotionClient {
    client: Client,
    token: String,
    settings: Settings,
}

pub struct RemoteWrite {
    pub id: String,
    pub url: String,
    pub edited_at: Option<String>,
}

impl NotionClient {
    pub fn new(token: String, settings: Settings) -> Result<Self, String> {
        let client = Client::builder()
            .user_agent("notion-tasks-local/0.3")
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client,
            token,
            settings,
        })
    }

    pub async fn validate(&self) -> Result<PropertyOptions, String> {
        let schema = self.fetch_schema().await?;
        validate_schema(&schema)?;

        let data_source_id = clean_id(&self.settings.data_source_id);
        let response = self
            .request(
                self.client
                    .post(format!("{NOTION_API}/data_sources/{data_source_id}/query")),
            )
            .json(&json!({ "page_size": 1 }))
            .send()
            .await
            .map_err(network_error)?;
        ensure_success(response).await?;
        Ok(property_options_from_schema(&schema))
    }

    pub async fn property_options(&self) -> Result<PropertyOptions, String> {
        let schema = self.fetch_schema().await?;
        Ok(property_options_from_schema(&schema))
    }

    async fn fetch_schema(&self) -> Result<Value, String> {
        let data_source_id = clean_id(&self.settings.data_source_id);
        let schema_response = self
            .request(
                self.client
                    .get(format!("{NOTION_API}/data_sources/{data_source_id}")),
            )
            .send()
            .await
            .map_err(network_error)?;
        ensure_success(schema_response).await
    }

    pub async fn push(&self, task: &Task) -> Result<RemoteWrite, String> {
        let properties = task_properties(task);
        let response = if let Some(page_id) = &task.notion_id {
            self.request(
                self.client
                    .patch(format!("{NOTION_API}/pages/{}", clean_id(page_id))),
            )
            .json(&json!({ "properties": properties }))
            .send()
            .await
            .map_err(network_error)?
        } else {
            self.request(self.client.post(format!("{NOTION_API}/pages")))
                .json(&json!({
                    "parent": { "data_source_id": clean_id(&self.settings.data_source_id) },
                    "properties": properties
                }))
                .send()
                .await
                .map_err(network_error)?
        };
        let value = ensure_success(response).await?;
        Ok(RemoteWrite {
            id: value
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            url: value
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            edited_at: value
                .get("last_edited_time")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }

    pub async fn delete(&self, page_id: &str) -> Result<(), String> {
        let response = self
            .request(
                self.client
                    .patch(format!("{NOTION_API}/pages/{}", clean_id(page_id))),
            )
            .json(&json!({ "in_trash": true }))
            .send()
            .await
            .map_err(network_error)?;
        ensure_success(response).await.map(|_| ())
    }

    pub async fn pull_all(&self) -> Result<Vec<(Task, Option<String>)>, String> {
        let mut output = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut body = json!({ "page_size": 100 });
            if let Some(value) = &cursor {
                body["start_cursor"] = json!(value);
            }
            let url = format!(
                "{NOTION_API}/data_sources/{}/query",
                clean_id(&self.settings.data_source_id)
            );
            let response = self
                .request(self.client.post(url))
                .json(&body)
                .send()
                .await
                .map_err(network_error)?;
            let value = ensure_success(response).await?;
            if let Some(results) = value.get("results").and_then(Value::as_array) {
                output.extend(results.iter().filter_map(page_to_task));
            }
            let has_more = value
                .get("has_more")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            cursor = value
                .get("next_cursor")
                .and_then(Value::as_str)
                .map(str::to_string);
            if !has_more || cursor.is_none() {
                break;
            }
        }
        Ok(output)
    }

    fn request(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder
            .bearer_auth(&self.token)
            .header("Notion-Version", NOTION_VERSION)
            .header("Content-Type", "application/json")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyOptions {
    pub source: Vec<String>,
    pub assignee: Vec<String>,
}

fn property_options_from_schema(schema: &Value) -> PropertyOptions {
    PropertyOptions {
        source: select_option_names(schema, "Source"),
        assignee: select_option_names(schema, "Resp."),
    }
}

pub fn normalize_select_name(value: &str) -> Result<String, String> {
    let name = value.trim();
    if name.contains(',') {
        return Err("Le nom du responsable ne peut pas contenir de virgule.".into());
    }
    Ok(name.to_string())
}

pub fn merge_option_names(mut remote: Vec<String>, keep: &[String]) -> Vec<String> {
    for name in keep {
        let name = name.trim();
        if name.is_empty() || remote.iter().any(|existing| existing == name) {
            continue;
        }
        remote.push(name.to_string());
    }
    remote
}

fn select_option_names(data_source: &Value, property: &str) -> Vec<String> {
    let Some(options) = data_source
        .get("properties")
        .and_then(|properties| properties.get(property))
        .and_then(|property| property.get("select"))
        .and_then(|select| select.get("options"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for option in options {
        let Some(name) = option.get("name").and_then(Value::as_str) else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() || names.iter().any(|existing: &String| existing == name) {
            continue;
        }
        names.push(name.to_string());
    }
    names
}

fn validate_schema(data_source: &Value) -> Result<(), String> {
    let properties = data_source
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| "Notion n’a pas renvoyé le schéma de la source de données.".to_string())?;
    let required = [
        ("Tâche", "title"),
        ("Done", "checkbox"),
        ("Statut", "select"),
        ("Priorité", "select"),
        ("Echéance", "date"),
        ("Fait le", "date"),
        ("Resp.", "select"),
        ("Source", "select"),
        ("description", "rich_text"),
        ("Commentaire", "rich_text"),
    ];
    let invalid: Vec<String> = required
        .iter()
        .filter_map(|(name, expected)| {
            let actual = properties
                .get(*name)
                .and_then(|value| value.get("type"))
                .and_then(Value::as_str);
            (actual != Some(*expected)).then(|| match actual {
                Some(actual) => format!("{name} ({actual}, attendu : {expected})"),
                None => format!("{name} (absente)"),
            })
        })
        .collect();
    if invalid.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Le schéma de la base Tâches ne correspond pas : {}.",
            invalid.join(", ")
        ))
    }
}

fn task_properties(task: &Task) -> Value {
    let mut properties = Map::new();
    properties.insert(
        "Tâche".into(),
        json!({ "title": [{ "text": { "content": task.title } }] }),
    );
    properties.insert("Done".into(), json!({ "checkbox": task.done }));
    properties.insert("Statut".into(), select_value(&task.status));
    properties.insert("Priorité".into(), select_value(&task.priority));
    properties.insert("Echéance".into(), date_value(task.due.as_deref()));
    properties.insert("Fait le".into(), date_value(task.completed_at.as_deref()));
    properties.insert("Resp.".into(), select_value(&task.assignee));
    properties.insert("Source".into(), select_value(&task.source));
    if !task.task_type.is_empty() {
        properties.insert("Type".into(), select_value(&task.task_type));
    }
    properties.insert("description".into(), rich_text_value(&task.description));
    properties.insert("Commentaire".into(), rich_text_value(&task.comment));
    Value::Object(properties)
}

fn select_value(value: &str) -> Value {
    if value.is_empty() {
        json!({ "select": null })
    } else {
        json!({ "select": { "name": value } })
    }
}

fn date_value(value: Option<&str>) -> Value {
    match value {
        Some(value) if !value.is_empty() => json!({ "date": { "start": value } }),
        _ => json!({ "date": null }),
    }
}

fn rich_text_value(value: &str) -> Value {
    if value.is_empty() {
        json!({ "rich_text": [] })
    } else {
        json!({ "rich_text": [{ "text": { "content": value } }] })
    }
}

fn page_to_task(page: &Value) -> Option<(Task, Option<String>)> {
    let properties = page.get("properties")?;
    let notion_id = page.get("id")?.as_str()?.to_string();
    let edited = page
        .get("last_edited_time")
        .and_then(Value::as_str)
        .map(str::to_string);
    let title = title_property(properties.get("Tâche")?);
    if title.trim().is_empty() {
        return None;
    }
    Some((
        Task {
            id: uuid::Uuid::new_v4().to_string(),
            notion_id: Some(notion_id),
            title,
            done: checkbox_property(properties.get("Done")),
            status: select_property(properties.get("Statut")),
            priority: select_property(properties.get("Priorité")),
            due: date_property(properties.get("Echéance")),
            completed_at: date_property(properties.get("Fait le")),
            assignee: select_property(properties.get("Resp.")),
            source: select_property(properties.get("Source")),
            task_type: select_property(properties.get("Type")),
            description: rich_text_property(properties.get("description")),
            comment: rich_text_property(properties.get("Commentaire")),
            notion_url: page.get("url").and_then(Value::as_str).map(str::to_string),
            dirty: false,
            sync_error: None,
            updated_at: edited.clone().unwrap_or_else(|| Utc::now().to_rfc3339()),
        },
        edited,
    ))
}

fn title_property(value: &Value) -> String {
    text_fragments(value.get("title"))
}

fn rich_text_property(value: Option<&Value>) -> String {
    value
        .map(|value| text_fragments(value.get("rich_text")))
        .unwrap_or_default()
}

fn text_fragments(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("plain_text").and_then(Value::as_str))
                .collect::<String>()
        })
        .unwrap_or_default()
}

fn checkbox_property(value: Option<&Value>) -> bool {
    value
        .and_then(|value| value.get("checkbox"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn select_property(value: Option<&Value>) -> String {
    value
        .and_then(|value| value.get("select"))
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn date_property(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|value| value.get("date"))
        .and_then(|value| value.get("start"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

async fn ensure_success(response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    let body = response.text().await.map_err(network_error)?;
    if status.is_success() {
        return serde_json::from_str(&body)
            .map_err(|error| format!("Réponse Notion invalide : {error}"));
    }
    let message = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| body.chars().take(300).collect());
    let friendly = match status {
        StatusCode::UNAUTHORIZED => "Le jeton Notion est invalide ou a expiré",
        StatusCode::FORBIDDEN => "Le jeton n’a pas accès à cette base Notion",
        StatusCode::NOT_FOUND => "La source de données Notion est introuvable ou non partagée",
        StatusCode::TOO_MANY_REQUESTS => {
            "Notion limite temporairement les requêtes ; réessaie dans un instant"
        }
        _ => "Notion a refusé la requête",
    };
    Err(format!("{friendly} ({status}) : {message}"))
}

fn network_error(error: reqwest::Error) -> String {
    if error.is_connect() || error.is_timeout() {
        "Connexion à Notion impossible. Les modifications restent enregistrées localement.".into()
    } else {
        format!("Erreur réseau : {error}")
    }
}

fn clean_id(value: &str) -> String {
    value
        .trim()
        .trim_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(value)
        .split('?')
        .next()
        .unwrap_or(value)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_task_to_existing_notion_property_names() {
        let mut task = Task::new("Préparer la réunion".into(), &Settings::default());
        task.due = Some("2026-11-10".into());
        task.description = "Préparer l’ordre du jour".into();
        let properties = task_properties(&task);

        assert_eq!(
            properties["Tâche"]["title"][0]["text"]["content"],
            "Préparer la réunion"
        );
        assert_eq!(properties["Statut"]["select"]["name"], "A faire");
        assert_eq!(properties["Echéance"]["date"]["start"], "2026-11-10");
        assert_eq!(
            properties["description"]["rich_text"][0]["text"]["content"],
            "Préparer l’ordre du jour"
        );
    }

    #[test]
    fn extracts_data_source_id_from_url() {
        assert_eq!(
            clean_id("https://www.notion.so/workspace/11111111-2222-3333-4444-555555555555?pvs=4"),
            "11111111-2222-3333-4444-555555555555"
        );
    }

    #[test]
    fn validates_expected_database_schema() {
        let properties = json!({
            "Tâche": { "type": "title" },
            "Done": { "type": "checkbox" },
            "Statut": { "type": "select" },
            "Priorité": { "type": "select" },
            "Echéance": { "type": "date" },
            "Fait le": { "type": "date" },
            "Resp.": { "type": "select" },
            "Source": { "type": "select" },
            "description": { "type": "rich_text" },
            "Commentaire": { "type": "rich_text" }
        });
        assert!(validate_schema(&json!({ "properties": properties })).is_ok());
    }

    #[test]
    fn extracts_source_options_in_notion_order() {
        let schema = json!({
            "properties": {
                "Source": {
                    "type": "select",
                    "select": {
                        "options": [
                            { "name": "Mail", "color": "blue" },
                            { "name": "  Réunion  " },
                            { "name": "" },
                            { "name": "Mail" },
                            { "color": "gray" }
                        ]
                    }
                }
            }
        });
        assert_eq!(
            select_option_names(&schema, "Source"),
            vec!["Mail".to_string(), "Réunion".to_string()]
        );
    }

    #[test]
    fn reads_source_and_assignee_options() {
        let schema = json!({
            "properties": {
                "Source": { "select": { "options": [{ "name": "Mail" }] } },
                "Resp.": { "select": { "options": [{ "name": "Alice" }, { "name": "Bob" }] } }
            }
        });
        assert_eq!(
            property_options_from_schema(&schema),
            PropertyOptions {
                source: vec!["Mail".into()],
                assignee: vec!["Alice".into(), "Bob".into()],
            }
        );
    }

    #[test]
    fn keeps_a_new_assignee_until_notion_returns_it() {
        let merged = merge_option_names(
            vec!["Alice".into()],
            &[" Bob ".into(), "Alice".into(), "".into()],
        );
        assert_eq!(merged, vec!["Alice".to_string(), "Bob".to_string()]);
    }

    #[test]
    fn rejects_an_assignee_name_containing_a_comma() {
        assert!(normalize_select_name("Ada, Lovelace").is_err());
        assert_eq!(normalize_select_name("  Ada  ").unwrap(), "Ada");
    }

    #[test]
    fn reports_a_missing_database_property() {
        let error = validate_schema(&json!({ "properties": {} })).expect_err("invalid schema");
        assert!(error.contains("Tâche (absente)"));
        assert!(error.contains("Done (absente)"));
    }
}
