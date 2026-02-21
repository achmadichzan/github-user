use serde::Deserialize;

/// Represents a GitHub user profile from the `/users/{username}` API.
#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    #[allow(dead_code)]
    pub id: u64,
    pub avatar_url: String,
    pub html_url: String,
    pub name: Option<String>,
    pub bio: Option<String>,
    pub public_repos: u32,
    pub followers: u32,
    pub following: u32,
    pub created_at: String,
}

/// Response from the GitHub Search Users API (`/search/users`).
#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub items: Vec<SearchUser>,
}

/// A single user item from the search results.
#[derive(Debug, Deserialize)]
pub struct SearchUser {
    pub login: String,
    #[allow(dead_code)]
    pub id: u64,
    pub avatar_url: String,
}
