mod db;
mod models;
mod notion;

use chrono::Utc;
use db::Database;
use keyring::Entry;
use models::{AppStatus, ConnectInput, Settings, SyncResult, Task};
use notion::NotionClient;
use std::sync::Arc;
use tauri::{Manager, State};

const KEYRING_SERVICE: &str = "fr.nicolascoutant.notiontasks";
const KEYRING_USER: &str = "notion-token";

struct AppState {
    db: Arc<Database>,
}

fn token_entry() -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|error| format!("Coffre système inaccessible : {error}"))
}

fn get_token() -> Result<Option<String>, String> {
    match token_entry()?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("Impossible de lire le jeton local : {error}")),
    }
}

fn build_status(db: &Database) -> Result<AppStatus, String> {
    let settings = db.settings()?;
    let token_stored = get_token()?.is_some();
    Ok(AppStatus {
        configured: token_stored && !settings.data_source_id.trim().is_empty(),
        token_stored,
        settings,
        pending_count: db.pending_count()?,
    })
}

#[tauri::command]
fn get_app_status(state: State<'_, AppState>) -> Result<AppStatus, String> {
    build_status(&state.db)
}

#[tauri::command]
fn list_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    state.db.list_tasks()
}

#[tauri::command]
fn create_task(title: String, state: State<'_, AppState>) -> Result<Task, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("Le titre de la tâche est vide.".into());
    }
    let settings = state.db.settings()?;
    let task = Task::new(title.to_string(), &settings);
    state.db.insert_local(&task)?;
    Ok(task)
}

#[tauri::command]
fn save_task(mut input: Task, state: State<'_, AppState>) -> Result<Task, String> {
    if input.title.trim().is_empty() {
        return Err("Le titre de la tâche est vide.".into());
    }
    input.title = input.title.trim().to_string();
    input.dirty = true;
    input.sync_error = None;
    input.updated_at = Utc::now().to_rfc3339();
    state.db.save_local(&input)?;
    Ok(input)
}

#[tauri::command]
fn complete_task(id: String, done: bool, state: State<'_, AppState>) -> Result<Task, String> {
    let mut task = state.db.task(&id)?;
    task.done = done;
    task.status = if done { "Terminé" } else { "A faire" }.into();
    task.completed_at = if done {
        Some(Utc::now().date_naive().to_string())
    } else {
        None
    };
    task.dirty = true;
    task.sync_error = None;
    task.updated_at = Utc::now().to_rfc3339();
    state.db.save_local(&task)?;
    Ok(task)
}

#[tauri::command]
fn save_settings(settings: Settings, state: State<'_, AppState>) -> Result<AppStatus, String> {
    state.db.save_settings(&settings)?;
    build_status(&state.db)
}

#[tauri::command]
async fn connect_notion(
    input: ConnectInput,
    state: State<'_, AppState>,
) -> Result<AppStatus, String> {
    if input.token.trim().is_empty() {
        return Err("Le jeton Notion est vide.".into());
    }
    if input.settings.data_source_id.trim().is_empty() {
        return Err("L’identifiant de la source de données est vide.".into());
    }
    let client = NotionClient::new(input.token.trim().to_string(), input.settings.clone())?;
    client.validate().await?;
    token_entry()?
        .set_password(input.token.trim())
        .map_err(|error| {
            format!("Impossible d’enregistrer le jeton dans le coffre système : {error}")
        })?;
    state.db.save_settings(&input.settings)?;
    build_status(&state.db)
}

#[tauri::command]
fn disconnect_notion(state: State<'_, AppState>) -> Result<AppStatus, String> {
    match token_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => build_status(&state.db),
        Err(error) => Err(format!("Impossible d’effacer le jeton : {error}")),
    }
}

#[tauri::command]
async fn sync_now(state: State<'_, AppState>) -> Result<SyncResult, String> {
    let token = get_token()?.ok_or_else(|| "Aucun jeton Notion n’est configuré.".to_string())?;
    let settings = state.db.settings()?;
    let client = NotionClient::new(token, settings)?;
    let dirty = state.db.dirty_tasks()?;
    let mut pushed = 0;
    let mut failed = 0;
    for task in dirty {
        match client.push(&task).await {
            Ok(remote) => {
                state.db.mark_synced(
                    &task.id,
                    &remote.id,
                    &remote.url,
                    remote.edited_at.as_deref(),
                )?;
                pushed += 1;
            }
            Err(error) => {
                state.db.mark_sync_error(&task.id, &error)?;
                failed += 1;
            }
        }
    }
    let mut pulled = 0;
    match client.pull_all().await {
        Ok(tasks) => {
            for (task, edited) in tasks {
                if state.db.upsert_remote(&task, edited.as_deref())? {
                    pulled += 1;
                }
            }
        }
        Err(error) if pushed == 0 && failed == 0 => return Err(error),
        Err(_) => failed += 1,
    }
    let pending = state.db.pending_count()?;
    let message = if failed == 0 {
        format!("Synchronisation terminée : {pushed} envoyée(s), {pulled} reçue(s).")
    } else {
        format!("Synchronisation partielle : {pushed} envoyée(s), {pulled} reçue(s), {failed} erreur(s).")
    };
    Ok(SyncResult {
        pushed,
        pulled,
        failed,
        pending,
        message,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
            std::fs::create_dir_all(&data_dir)?;
            let db = Database::new(data_dir.join("tasks.sqlite"))?;
            app.manage(AppState { db: Arc::new(db) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            list_tasks,
            create_task,
            save_task,
            complete_task,
            save_settings,
            connect_notion,
            disconnect_notion,
            sync_now,
        ])
        .run(tauri::generate_context!())
        .expect("échec au lancement de l’application");
}
