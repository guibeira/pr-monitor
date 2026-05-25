use crate::error::CredentialError;
use keyring::{Entry, Error as KeyringError};
use std::sync::Arc;
use tokio::sync::Mutex;

const SERVICE_NAME: &str = "pr-monitor";
const GITHUB_TOKEN_USER: &str = "github-token";

type CredentialResult<T> = Result<T, CredentialError>;

#[derive(Clone, Default)]
pub struct CredentialStore {
    access: Arc<Mutex<()>>,
}

impl CredentialStore {
    pub async fn set_github_token(&self, token: String) -> CredentialResult<()> {
        let _guard = self.access.lock().await;
        tokio::task::spawn_blocking(move || -> CredentialResult<()> {
            github_token_entry()?.set_password(&token)?;
            Ok(())
        })
        .await?
    }

    pub async fn get_github_token(&self) -> CredentialResult<Option<String>> {
        let _guard = self.access.lock().await;
        tokio::task::spawn_blocking(move || -> CredentialResult<Option<String>> {
            match github_token_entry()?.get_password() {
                Ok(token) => Ok(Some(token)),
                Err(KeyringError::NoEntry) => Ok(None),
                Err(err) => Err(err.into()),
            }
        })
        .await?
    }
}

fn github_token_entry() -> CredentialResult<Entry> {
    Ok(Entry::new(SERVICE_NAME, GITHUB_TOKEN_USER)?)
}
