use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestKey {
    pub owner: String,
    pub repo: String,
    pub pr_number: u64,
}

impl PullRequestKey {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>, pr_number: u64) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            pr_number,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PullRequestState {
    Open,
    Closed,
}

impl PullRequestState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
        }
    }
}

impl fmt::Display for PullRequestState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for PullRequestState {
    type Error = PullRequestStateParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "open" => Ok(Self::Open),
            "closed" => Ok(Self::Closed),
            other => Err(PullRequestStateParseError(other.to_owned())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid pull request state: {0}")]
pub struct PullRequestStateParseError(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestModel {
    pub owner: String,
    pub repo: String,
    pub pr_number: u64,
    pub title: String,
    pub state: PullRequestState,
    pub closed_at: Option<String>,
    pub url: String,
}

impl PullRequestModel {
    pub fn key(&self) -> PullRequestKey {
        PullRequestKey::new(&self.owner, &self.repo, self.pr_number)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrStatus {
    Merged,
    Behind,
    UpToDate,
    Conflicts,
    Blocked,
    Unknown,
}
