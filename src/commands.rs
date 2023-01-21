use crate::{release, repo};
use anyhow::Result;
use octocrab::Octocrab;

/// Lists releases for a repo.
pub async fn list(client: Octocrab, repo_name: &str, include_drafts: &bool) -> Result<()> {
    println!("listing releases for {}...", repo_name);

    let repo = repo::parse(repo_name)?;

    let releases = client
        .repos(repo.owner, repo.name)
        .releases()
        .list()
        .per_page(20)
        .send()
        .await?;

    println!("--------------------");

    for release in releases.items {
        if release.draft && !*include_drafts {
            continue;
        }

        println!(
            "name: {}\ntag: {}\n--------------------",
            release.name.unwrap_or(String::from("n/a")),
            release.tag_name,
        );
    }

    Ok(())
}

/// Installs the defined repo release.
pub async fn install(client: Octocrab, repo_name: &str, _include_drafts: &bool) -> Result<()> {
    let repo = repo::parse(repo_name)?;

    release::install(client, &repo).await?;

    Ok(())
}
