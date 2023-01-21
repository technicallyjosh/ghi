use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Repo {
    pub(crate) owner: String,
    pub(crate) name: String,
    pub(crate) tag: String,
    pub(crate) full_name: String,
}

impl Repo {
    pub fn create_dir(self: &Repo) -> Result<()> {
        fs::create_dir_all(self.dir())?;
        Ok(())
    }

    pub fn dir(self: &Repo) -> PathBuf {
        return dirs::home_dir()
            .unwrap()
            .join(".ghi")
            .join(&self.owner)
            .join(&self.name)
            .join(&self.tag);
    }
}

/// Parses a repo string into a usable struct.
pub fn parse(repo: &str) -> Result<Repo> {
    let meta_parts: Vec<&str> = repo.split("@").collect();
    let repo_parts: Vec<&str> = meta_parts[0].split("/").collect();

    if repo_parts.len() != 2 {
        // We should only have {owner}/{repo}
        bail!("Repo must have 2 parts e.g. technicallyjosh/ghi");
    }

    if meta_parts.len() > 2 {
        // We should only have {repo}@{tag} or {repo}
        bail!("Cannot define more than the repo and tag");
    } else if meta_parts.len() == 2 {
        if meta_parts[1].trim() == "" {
            bail!("Tag cannot be empty");
        }
    }

    let tag = if meta_parts.len() == 2 {
        meta_parts[1]
    } else {
        "latest"
    };

    Ok(Repo {
        owner: repo_parts[0].to_string(),
        name: repo_parts[1].to_string(),
        tag: tag.to_string(),
        full_name: format!("{}/{}/{}", repo_parts[0], repo_parts[1], tag),
    })
}
