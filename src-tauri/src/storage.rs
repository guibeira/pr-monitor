use crate::domain::{PullRequestKey, PullRequestModel, PullRequestState};
use crate::error::StorageError;
use rusqlite::types::Type;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

type StorageResult<T> = Result<T, StorageError>;

#[derive(Clone)]
pub struct Storage {
    conn: Arc<Mutex<Connection>>,
}

impl Storage {
    pub fn new(db_path: PathBuf) -> StorageResult<Self> {
        let conn = Connection::open(db_path)?;
        Self::migrate(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn migrate(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pull_request (
                id INTEGER PRIMARY KEY,
                owner TEXT NOT NULL,
                repo TEXT NOT NULL,
                pr_number INTEGER NOT NULL,
                title TEXT NOT NULL,
                state TEXT NOT NULL,
                url TEXT NOT NULL,
                closed_at TEXT
            );",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
            [],
        )?;
        conn.execute("DROP TABLE IF EXISTS token;", [])?;
        conn.execute(
            "DELETE FROM pull_request
             WHERE id NOT IN (
                SELECT MIN(id)
                FROM pull_request
                GROUP BY owner, repo, pr_number
             );",
            [],
        )?;
        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_pull_request_identity
             ON pull_request(owner, repo, pr_number);",
            [],
        )?;

        Ok(())
    }

    async fn with_conn<T, F>(&self, f: F) -> StorageResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> StorageResult<T> + Send + 'static,
    {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| StorageError::LockPoisoned)?;
            f(&conn)
        })
        .await?
    }

    pub async fn get_theme(&self) -> StorageResult<String> {
        self.get_setting("theme", "system").await
    }

    pub async fn set_theme(&self, theme: String) -> StorageResult<()> {
        self.set_setting("theme", theme).await
    }

    pub async fn get_refresh_time(&self) -> StorageResult<u64> {
        let value = self.get_setting("refresh_time", "300").await?;
        Ok(value.parse::<u64>().unwrap_or(300))
    }

    pub async fn set_refresh_time(&self, time_in_seconds: u64) -> StorageResult<()> {
        self.set_setting("refresh_time", time_in_seconds.to_string())
            .await
    }

    pub async fn get_show_notification(&self) -> StorageResult<bool> {
        let value = self.get_setting("show_notification", "true").await?;
        Ok(value.parse::<bool>().unwrap_or(true))
    }

    pub async fn set_show_notification(&self, show: bool) -> StorageResult<()> {
        self.set_setting("show_notification", show.to_string())
            .await
    }

    async fn get_setting(&self, key: &'static str, default: &'static str) -> StorageResult<String> {
        self.with_conn(move |conn| {
            let value = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = ?",
                    params![key],
                    |row| row.get(0),
                )
                .optional()?;

            Ok(value.unwrap_or_else(|| default.to_owned()))
        })
        .await
    }

    async fn set_setting(&self, key: &'static str, value: String) -> StorageResult<()> {
        self.with_conn(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
                params![key, value],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn add_pull_request(&self, pull_request: PullRequestModel) -> StorageResult<()> {
        self.with_conn(move |conn| {
            let exists: i64 = conn.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM pull_request
                    WHERE owner = ? AND repo = ? AND pr_number = ?
                )",
                params![
                    &pull_request.owner,
                    &pull_request.repo,
                    pull_request.pr_number
                ],
                |row| row.get(0),
            )?;

            if exists != 0 {
                return Err(StorageError::PullRequestAlreadyExists);
            }

            conn.execute(
                "INSERT INTO pull_request (owner, repo, pr_number, title, state, url, closed_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![
                    pull_request.owner,
                    pull_request.repo,
                    pull_request.pr_number,
                    pull_request.title,
                    pull_request.state.as_str(),
                    pull_request.url,
                    pull_request.closed_at
                ],
            )?;

            Ok(())
        })
        .await
    }

    pub async fn delete_pull_request(&self, key: PullRequestKey) -> StorageResult<()> {
        self.with_conn(move |conn| {
            conn.execute(
                "DELETE FROM pull_request
                 WHERE owner = ? AND repo = ? AND pr_number = ?",
                params![key.owner, key.repo, key.pr_number],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn mark_pull_request_closed(&self, key: PullRequestKey) -> StorageResult<()> {
        self.with_conn(move |conn| {
            conn.execute(
                "UPDATE pull_request
                 SET state = ?
                 WHERE owner = ? AND repo = ? AND pr_number = ?",
                params![
                    PullRequestState::Closed.as_str(),
                    key.owner,
                    key.repo,
                    key.pr_number
                ],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn get_open_pull_requests(&self) -> StorageResult<Vec<PullRequestModel>> {
        self.list_pull_requests(Some(PullRequestState::Open)).await
    }

    pub async fn get_all_pull_requests(&self) -> StorageResult<Vec<PullRequestModel>> {
        self.list_pull_requests(None).await
    }

    async fn list_pull_requests(
        &self,
        state: Option<PullRequestState>,
    ) -> StorageResult<Vec<PullRequestModel>> {
        self.with_conn(move |conn| {
            let mut pull_requests = Vec::new();

            if let Some(state) = state {
                let mut stmt = conn.prepare(
                    "SELECT owner, repo, pr_number, title, state, closed_at, url
                     FROM pull_request
                     WHERE state = ?
                     ORDER BY owner, repo, pr_number",
                )?;
                let mut rows = stmt.query(params![state.as_str()])?;
                while let Some(row) = rows.next()? {
                    pull_requests.push(row_to_pull_request(row)?);
                }
            } else {
                let mut stmt = conn.prepare(
                    "SELECT owner, repo, pr_number, title, state, closed_at, url
                     FROM pull_request
                     ORDER BY owner, repo, pr_number",
                )?;
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    pull_requests.push(row_to_pull_request(row)?);
                }
            }

            Ok(pull_requests)
        })
        .await
    }
}

fn row_to_pull_request(row: &rusqlite::Row<'_>) -> rusqlite::Result<PullRequestModel> {
    let state_text: String = row.get(4)?;
    let state = PullRequestState::try_from(state_text.as_str())
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(4, Type::Text, Box::new(err)))?;

    Ok(PullRequestModel {
        owner: row.get(0)?,
        repo: row.get(1)?,
        pr_number: row.get(2)?,
        title: row.get(3)?,
        state,
        closed_at: row.get(5)?,
        url: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("pr-monitor-{name}-{nanos}.db"))
    }

    fn pull_request(owner: &str, repo: &str, pr_number: u64) -> PullRequestModel {
        PullRequestModel {
            owner: owner.to_owned(),
            repo: repo.to_owned(),
            pr_number,
            title: format!("{owner}/{repo}#{pr_number}"),
            state: PullRequestState::Open,
            closed_at: None,
            url: format!("https://api.github.com/repos/{owner}/{repo}/pulls/{pr_number}"),
        }
    }

    #[tokio::test]
    async fn settings_return_defaults_when_not_configured() {
        let storage = Storage::new(temp_db_path("defaults")).expect("storage should initialize");

        assert_eq!(storage.get_theme().await.unwrap(), "system");
        assert_eq!(storage.get_refresh_time().await.unwrap(), 300);
        assert!(storage.get_show_notification().await.unwrap());
    }

    #[tokio::test]
    async fn duplicate_pull_requests_are_scoped_by_repository() {
        let storage = Storage::new(temp_db_path("identity")).expect("storage should initialize");

        storage
            .add_pull_request(pull_request("owner-a", "repo", 7))
            .await
            .unwrap();
        storage
            .add_pull_request(pull_request("owner-b", "repo", 7))
            .await
            .unwrap();

        let duplicate = storage
            .add_pull_request(pull_request("owner-a", "repo", 7))
            .await
            .unwrap_err();

        assert!(matches!(duplicate, StorageError::PullRequestAlreadyExists));
    }

    #[tokio::test]
    async fn delete_pull_request_uses_repository_identity() {
        let storage = Storage::new(temp_db_path("delete")).expect("storage should initialize");

        storage
            .add_pull_request(pull_request("owner-a", "repo", 7))
            .await
            .unwrap();
        storage
            .add_pull_request(pull_request("owner-b", "repo", 7))
            .await
            .unwrap();

        storage
            .delete_pull_request(PullRequestKey::new("owner-a", "repo", 7))
            .await
            .unwrap();

        let remaining = storage.get_all_pull_requests().await.unwrap();

        assert_eq!(remaining, vec![pull_request("owner-b", "repo", 7)]);
    }
}
