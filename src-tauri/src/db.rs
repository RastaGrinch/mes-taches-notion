use crate::models::{Settings, Task};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, String> {
        let db = Self {
            path: path.as_ref().to_path_buf(),
        };
        db.initialize()?;
        Ok(db)
    }

    fn connect(&self) -> Result<Connection, String> {
        let connection = Connection::open(&self.path).map_err(|error| error.to_string())?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| error.to_string())?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(|error| error.to_string())?;
        Ok(connection)
    }

    fn initialize(&self) -> Result<(), String> {
        let connection = self.connect()?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                notion_id TEXT UNIQUE,
                title TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT '',
                priority TEXT NOT NULL DEFAULT '',
                due TEXT,
                completed_at TEXT,
                assignee TEXT NOT NULL DEFAULT '',
                source TEXT NOT NULL DEFAULT '',
                task_type TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT '',
                comment TEXT NOT NULL DEFAULT '',
                notion_url TEXT,
                remote_edited_at TEXT,
                dirty INTEGER NOT NULL DEFAULT 0,
                sync_error TEXT,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS outbox (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id TEXT NOT NULL UNIQUE,
                operation TEXT NOT NULL DEFAULT 'upsert',
                attempts INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS deletions (
                task_id TEXT PRIMARY KEY,
                notion_id TEXT NOT NULL UNIQUE,
                attempts INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_tasks_notion_id ON tasks(notion_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_done ON tasks(done);
            CREATE INDEX IF NOT EXISTS idx_outbox_task ON outbox(task_id);
            CREATE INDEX IF NOT EXISTS idx_deletions_notion_id ON deletions(notion_id);",
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn settings(&self) -> Result<Settings, String> {
        let connection = self.connect()?;
        let raw: Option<String> = connection
            .query_row("SELECT value FROM settings WHERE key = 'app'", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|error| error.to_string())?;
        match raw {
            Some(value) => serde_json::from_str(&value).map_err(|error| error.to_string()),
            None => Ok(Settings::default()),
        }
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        let connection = self.connect()?;
        let value = serde_json::to_string(settings).map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT INTO settings(key, value) VALUES('app', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [value],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn list_tasks(&self) -> Result<Vec<Task>, String> {
        let connection = self.connect()?;
        let mut statement = connection
            .prepare(
                "SELECT id, notion_id, title, done, status, priority, due, completed_at,
                    assignee, source, task_type, description, comment, notion_url,
                    dirty, sync_error, updated_at
             FROM tasks ORDER BY updated_at DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], task_from_row)
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn task(&self, id: &str) -> Result<Task, String> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT id, notion_id, title, done, status, priority, due, completed_at,
                    assignee, source, task_type, description, comment, notion_url,
                    dirty, sync_error, updated_at FROM tasks WHERE id = ?1",
                [id],
                task_from_row,
            )
            .map_err(|error| error.to_string())
    }

    pub fn dirty_tasks(&self) -> Result<Vec<Task>, String> {
        let connection = self.connect()?;
        let mut statement = connection
            .prepare(
                "SELECT t.id, t.notion_id, t.title, t.done, t.status, t.priority, t.due,
                    t.completed_at, t.assignee, t.source, t.task_type, t.description,
                    t.comment, t.notion_url, t.dirty, t.sync_error, t.updated_at
             FROM tasks t INNER JOIN outbox o ON o.task_id = t.id ORDER BY o.id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], task_from_row)
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn insert_local(&self, task: &Task) -> Result<(), String> {
        let connection = self.connect()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        upsert_task(&transaction, task, None)?;
        enqueue(&transaction, &task.id)?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub fn save_local(&self, task: &Task) -> Result<(), String> {
        let connection = self.connect()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        upsert_task(&transaction, task, None)?;
        enqueue(&transaction, &task.id)?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub fn delete_local(&self, task_id: &str) -> Result<(), String> {
        let connection = self.connect()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        let notion_id: Option<Option<String>> = transaction
            .query_row(
                "SELECT notion_id FROM tasks WHERE id = ?1",
                [task_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let notion_id = notion_id.ok_or_else(|| "La tâche est introuvable.".to_string())?;
        if let Some(notion_id) = notion_id {
            transaction
                .execute(
                    "INSERT INTO deletions(task_id, notion_id, created_at) VALUES(?1, ?2, ?3)
                     ON CONFLICT(task_id) DO UPDATE SET notion_id = excluded.notion_id,
                         attempts = 0, last_error = NULL",
                    params![task_id, notion_id, chrono::Utc::now().to_rfc3339()],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction
            .execute("DELETE FROM tasks WHERE id = ?1", [task_id])
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub fn pending_deletions(&self) -> Result<Vec<(String, String)>, String> {
        let connection = self.connect()?;
        let mut statement = connection
            .prepare("SELECT task_id, notion_id FROM deletions ORDER BY created_at")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn mark_deletion_synced(&self, task_id: &str) -> Result<(), String> {
        let connection = self.connect()?;
        connection
            .execute("DELETE FROM deletions WHERE task_id = ?1", [task_id])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn mark_deletion_error(&self, task_id: &str, message: &str) -> Result<(), String> {
        let connection = self.connect()?;
        connection
            .execute(
                "UPDATE deletions SET attempts = attempts + 1, last_error = ?2 WHERE task_id = ?1",
                params![task_id, message],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn mark_synced(
        &self,
        local_id: &str,
        notion_id: &str,
        notion_url: &str,
        remote_edited_at: Option<&str>,
    ) -> Result<(), String> {
        let connection = self.connect()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE tasks SET notion_id = ?2, notion_url = ?3, remote_edited_at = ?4,
                    dirty = 0, sync_error = NULL WHERE id = ?1",
                params![local_id, notion_id, notion_url, remote_edited_at],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM outbox WHERE task_id = ?1", [local_id])
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub fn mark_sync_error(&self, local_id: &str, message: &str) -> Result<(), String> {
        let connection = self.connect()?;
        connection
            .execute(
                "UPDATE tasks SET sync_error = ?2 WHERE id = ?1",
                params![local_id, message],
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE outbox SET attempts = attempts + 1, last_error = ?2 WHERE task_id = ?1",
                params![local_id, message],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn upsert_remote(
        &self,
        task: &Task,
        remote_edited_at: Option<&str>,
    ) -> Result<bool, String> {
        let connection = self.connect()?;
        let pending_deletion = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM deletions WHERE notion_id = ?1)",
                [task.notion_id.as_deref().unwrap_or_default()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| error.to_string())?
            != 0;
        if pending_deletion {
            return Ok(false);
        }
        let local: Option<(String, bool)> = connection
            .query_row(
                "SELECT id, dirty FROM tasks WHERE notion_id = ?1",
                [task.notion_id.as_deref().unwrap_or_default()],
                |row| Ok((row.get(0)?, row.get::<_, i64>(1)? != 0)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if let Some((_, true)) = local {
            return Ok(false);
        }
        let mut remote = task.clone();
        if let Some((local_id, _)) = local {
            remote.id = local_id;
        }
        upsert_task(&connection, &remote, remote_edited_at)?;
        Ok(true)
    }

    pub fn pending_count(&self) -> Result<usize, String> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM outbox) + (SELECT COUNT(*) FROM deletions)",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|value| value as usize)
            .map_err(|error| error.to_string())
    }
}

fn enqueue(connection: &Connection, task_id: &str) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO outbox(task_id, operation, created_at) VALUES(?1, 'upsert', ?2)
         ON CONFLICT(task_id) DO UPDATE SET operation = 'upsert', last_error = NULL",
            params![task_id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn upsert_task(
    connection: &Connection,
    task: &Task,
    remote_edited_at: Option<&str>,
) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO tasks(
            id, notion_id, title, done, status, priority, due, completed_at, assignee,
            source, task_type, description, comment, notion_url, remote_edited_at,
            dirty, sync_error, updated_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
         ON CONFLICT(id) DO UPDATE SET
            notion_id=excluded.notion_id, title=excluded.title, done=excluded.done,
            status=excluded.status, priority=excluded.priority, due=excluded.due,
            completed_at=excluded.completed_at, assignee=excluded.assignee,
            source=excluded.source, task_type=excluded.task_type,
            description=excluded.description, comment=excluded.comment,
            notion_url=excluded.notion_url,
            remote_edited_at=COALESCE(excluded.remote_edited_at, tasks.remote_edited_at),
            dirty=excluded.dirty, sync_error=excluded.sync_error, updated_at=excluded.updated_at",
            params![
                task.id,
                task.notion_id,
                task.title,
                task.done as i64,
                task.status,
                task.priority,
                task.due,
                task.completed_at,
                task.assignee,
                task.source,
                task.task_type,
                task.description,
                task.comment,
                task.notion_url,
                remote_edited_at,
                task.dirty as i64,
                task.sync_error,
                task.updated_at,
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn task_from_row(row: &Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        notion_id: row.get(1)?,
        title: row.get(2)?,
        done: row.get::<_, i64>(3)? != 0,
        status: row.get(4)?,
        priority: row.get(5)?,
        due: row.get(6)?,
        completed_at: row.get(7)?,
        assignee: row.get(8)?,
        source: row.get(9)?,
        task_type: row.get(10)?,
        description: row.get(11)?,
        comment: row.get(12)?,
        notion_url: row.get(13)?,
        dirty: row.get::<_, i64>(14)? != 0,
        sync_error: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_task_survives_restart_and_leaves_outbox_after_sync() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("tasks.sqlite");
        let db = Database::new(&path).expect("database");
        let task = Task::new("Tester la saisie rapide".into(), &Settings::default());

        db.insert_local(&task).expect("insert task");
        assert_eq!(db.pending_count().expect("pending count"), 1);
        assert_eq!(
            db.list_tasks().expect("task list")[0].title,
            "Tester la saisie rapide"
        );

        drop(db);
        let reopened = Database::new(&path).expect("reopen database");
        assert_eq!(reopened.pending_count().expect("pending after restart"), 1);
        reopened
            .mark_synced(&task.id, "page-id", "https://notion.so/page-id", None)
            .expect("mark synced");
        assert_eq!(reopened.pending_count().expect("empty outbox"), 0);
        let saved = reopened.task(&task.id).expect("saved task");
        assert!(!saved.dirty);
        assert_eq!(saved.notion_id.as_deref(), Some("page-id"));
    }

    #[test]
    fn deleting_a_local_only_task_removes_its_pending_upsert() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let db = Database::new(directory.path().join("tasks.sqlite")).expect("database");
        let task = Task::new("Tâche locale".into(), &Settings::default());

        db.insert_local(&task).expect("insert task");
        db.delete_local(&task.id).expect("delete task");

        assert!(db.list_tasks().expect("task list").is_empty());
        assert!(db.pending_deletions().expect("deletions").is_empty());
        assert_eq!(db.pending_count().expect("pending count"), 0);
    }

    #[test]
    fn deleting_a_synced_task_waits_for_remote_deletion() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let db = Database::new(directory.path().join("tasks.sqlite")).expect("database");
        let task = Task::new("Tâche synchronisée".into(), &Settings::default());

        db.insert_local(&task).expect("insert task");
        db.mark_synced(&task.id, "page-id", "https://notion.so/page-id", None)
            .expect("mark synced");
        db.delete_local(&task.id).expect("delete task");

        assert!(db.list_tasks().expect("task list").is_empty());
        assert_eq!(
            db.pending_deletions().expect("deletions"),
            vec![(task.id.clone(), "page-id".into())]
        );
        assert_eq!(db.pending_count().expect("pending count"), 1);

        db.mark_deletion_synced(&task.id)
            .expect("mark deletion synced");
        assert_eq!(db.pending_count().expect("pending count"), 0);
    }
}
