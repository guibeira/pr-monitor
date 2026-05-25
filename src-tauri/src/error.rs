use crate::domain::PullRequestStateParseError;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Github(#[from] GithubError),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error("There is no token, can't check pr details")]
    MissingToken,

    #[error("Failed to parse PR")]
    InvalidPullRequestUrl,

    #[error("window `{0}` was not found")]
    WindowNotFound(&'static str),

    #[error("failed to emit app event: {0}")]
    Event(String),

    #[error("failed to show notification: {0}")]
    Notification(String),
}

#[derive(Debug, thiserror::Error)]
pub enum GithubError {
    #[error("Failed to create GitHub client: {0}")]
    Client(#[source] Box<octocrab::Error>),

    #[error("GitHub request failed: {0}")]
    Request(#[source] Box<octocrab::Error>),

    #[error("Can't load pr details")]
    PullRequestDetailsUnavailable,

    #[error("Can't update pr branch")]
    UpdateBranchUnavailable,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("database worker failed: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("database lock is poisoned")]
    LockPoisoned,

    #[error("Pull request already exists")]
    PullRequestAlreadyExists,

    #[error(transparent)]
    InvalidPullRequestState(#[from] PullRequestStateParseError),
}
