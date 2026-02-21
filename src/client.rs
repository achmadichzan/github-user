use anyhow::{Context, Result};
use reqwest::Client;

use crate::models::{GitHubUser, SearchUser};

/// Creates a preconfigured HTTP client with required headers.
fn build_client(token: Option<&str>) -> Result<Client> {
    use reqwest::header::{HeaderMap, HeaderValue};

    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", HeaderValue::from_static("rust-github-user-client"));
    headers.insert("Accept", HeaderValue::from_static("application/vnd.github.v3+json"));

    let mut builder = Client::builder().default_headers(headers);

    if let Some(token) = token {
        use reqwest::header::AUTHORIZATION;
        let val = HeaderValue::from_str(&format!("Bearer {token}"))
            .context("Invalid token value")?;
        let mut auth = HeaderMap::new();
        auth.insert(AUTHORIZATION, val);
        builder = builder.default_headers(auth);
    }

    builder.build().context("Failed to build HTTP client")
}

/// Searches GitHub users by query string.
/// Returns up to 30 results sorted by best match.
pub async fn search_users(query: &str, token: Option<&str>) -> Result<Vec<SearchUser>> {
    let client = build_client(token)?;

    let url = format!(
        "https://api.github.com/search/users?q={}&per_page=30",
        urlencoding(query)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send search request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("GitHub Search API error ({status}): {body}");
    }

    let search: crate::models::SearchResponse = response
        .json()
        .await
        .context("Failed to deserialize search response")?;

    Ok(search.items)
}

/// Fetches a GitHub user by username.
pub async fn fetch_user(username: &str, token: Option<&str>) -> Result<GitHubUser> {
    let client = build_client(token)?;
    let url = format!("https://api.github.com/users/{username}");

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send request to GitHub API")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("GitHub API error ({status}): {body}");
    }

    let user = response
        .json::<GitHubUser>()
        .await
        .context("Failed to deserialize GitHub user response")?;

    Ok(user)
}

/// Simple percent-encoding for the query parameter.
fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "+".to_string(),
            c if c.is_ascii_alphanumeric() || "-._~".contains(c) => c.to_string(),
            c => format!("%{:02X}", c as u32),
        })
        .collect()
}
