use anyhow::{bail, Result};
use octocrab::Octocrab;

/// Returns a new GH client.
pub fn new_client(token: &str) -> Result<Octocrab> {
    let gh_result = if token == "" {
        Octocrab::builder().build()
    } else {
        Octocrab::builder()
            .personal_token(token.to_string())
            .build()
    };

    return match gh_result {
        Ok(gh) => Ok(gh),
        Err(err) => bail!("failed to create gh client: {}", err),
    };
}

/// Returns a token from the environment variable "GITHUB_TOKEN" if it exists.
/// This is then used by new_client to create a client for private repo access.
pub fn get_token() -> String {
    return std::env::var("GITHUB_TOKEN").unwrap_or("".to_string());
}
