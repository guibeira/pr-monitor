use crate::domain::{PrStatus, PullRequestKey, PullRequestModel, PullRequestState};
use crate::error::GithubError;
use log::error;
use octocrab::models::pulls::MergeableState;
use octocrab::Octocrab;
use regex::Regex;
use std::sync::LazyLock;

static GITHUB_PR_URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^https?://(?:www\.)?github\.com/([^/?#]+)/([^/?#]+)/pull/(\d+)/?(?:[?#].*)?$")
        .expect("static GitHub PR URL regex should be valid")
});

type GithubResult<T> = Result<T, GithubError>;

pub fn parse_github_pr_url(url: &str) -> Option<PullRequestKey> {
    let caps = GITHUB_PR_URL_REGEX.captures(url)?;

    let owner = caps.get(1)?.as_str().to_owned();
    let repo = caps.get(2)?.as_str().to_owned();
    let pr_number = caps.get(3)?.as_str().parse().ok()?;

    Some(PullRequestKey::new(owner, repo, pr_number))
}

pub async fn update_pr_branch(key: &PullRequestKey, token: &str) -> GithubResult<()> {
    let octocrab = Octocrab::builder()
        .personal_token(token.to_owned())
        .build()
        .map_err(|err| GithubError::Client(Box::new(err)))?;

    octocrab
        .pulls(&key.owner, &key.repo)
        .update_branch(key.pr_number)
        .await
        .map_err(|err| {
            error!("Error: {err:?}");
            GithubError::UpdateBranchUnavailable
        })?;

    Ok(())
}

pub async fn needs_update_pr(key: &PullRequestKey, token: &str) -> GithubResult<PrStatus> {
    let octocrab = Octocrab::builder()
        .personal_token(token.to_owned())
        .build()
        .map_err(|err| GithubError::Client(Box::new(err)))?;

    let pr = octocrab
        .pulls(&key.owner, &key.repo)
        .get(key.pr_number)
        .await
        .map_err(|err| {
            error!("Error: {err:?}");
            GithubError::Request(Box::new(err))
        })?;

    if pr.merged_at.is_some() {
        log::info!("PR was merged, we not need to update the branch");
        return Ok(PrStatus::Merged);
    }

    let status = match pr.mergeable_state {
        Some(MergeableState::Behind) => {
            log::info!("PR is behind, we need to update the branch");
            PrStatus::Behind
        }
        Some(MergeableState::Clean) => PrStatus::UpToDate,
        Some(MergeableState::Dirty) => PrStatus::Conflicts,
        Some(MergeableState::Blocked) => PrStatus::Blocked,
        Some(MergeableState::Unknown | MergeableState::Unstable) | None => PrStatus::Unknown,
        Some(_) => PrStatus::Unknown,
    };

    Ok(status)
}

pub async fn get_pr_details(token: &str, key: &PullRequestKey) -> GithubResult<PullRequestModel> {
    let octocrab = Octocrab::builder()
        .personal_token(token.to_owned())
        .build()
        .map_err(|err| GithubError::Client(Box::new(err)))?;

    let pr = octocrab
        .pulls(&key.owner, &key.repo)
        .get(key.pr_number)
        .await
        .map_err(|err| {
            error!("Error: {err:?}");
            GithubError::PullRequestDetailsUnavailable
        })?;

    if pr.mergeable_state.is_none() {
        return Err(GithubError::PullRequestDetailsUnavailable);
    }

    let state = match pr.state {
        Some(octocrab::models::IssueState::Open) => PullRequestState::Open,
        Some(octocrab::models::IssueState::Closed) => PullRequestState::Closed,
        _ => PullRequestState::Open,
    };

    Ok(PullRequestModel {
        owner: key.owner.clone(),
        repo: key.repo.clone(),
        pr_number: key.pr_number,
        title: pr.title.unwrap_or_default(),
        state,
        closed_at: pr
            .closed_at
            .map(|closed_at| closed_at.format("%d/%m/%Y %H:%M").to_string()),
        url: pr.url.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_github_pr_url_returns_repository_identity() {
        let key = parse_github_pr_url("https://github.com/acme/widgets/pull/42")
            .expect("url should parse");

        assert_eq!(key, PullRequestKey::new("acme", "widgets", 42));
    }

    #[test]
    fn parse_github_pr_url_allows_http_www_query_and_fragment() {
        let key = parse_github_pr_url("http://www.github.com/acme/widgets/pull/42?foo=bar#files")
            .expect("url should parse");

        assert_eq!(key, PullRequestKey::new("acme", "widgets", 42));
    }

    #[test]
    fn parse_github_pr_url_rejects_non_pull_request_urls() {
        let key = parse_github_pr_url("https://github.com/acme/widgets/issues/42");

        assert_eq!(key, None);
    }

    #[test]
    fn parse_github_pr_url_rejects_urls_embedded_in_other_text() {
        let key = parse_github_pr_url("see https://github.com/acme/widgets/pull/42");

        assert_eq!(key, None);
    }

    #[test]
    fn parse_github_pr_url_rejects_non_github_hosts() {
        let key = parse_github_pr_url("https://example.com/github.com/acme/widgets/pull/42");

        assert_eq!(key, None);
    }
}
