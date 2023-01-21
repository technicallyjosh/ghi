use anyhow::Result;
use clap::Parser;
use dirs;
use std::fs;

mod commands;
mod gh;
mod release;
mod repo;

#[derive(Parser)]
#[command(version)]
struct CLI {
    /// Repository to use. e.g. technicallyjosh/ghi
    repo: String,

    /// List releases in the repository
    #[arg(long, short, default_value_t = false)]
    list: bool,

    /// Include draft releases when listing
    #[arg(long, short = 'd', default_value_t = false)]
    include_drafts: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = CLI::parse();

    let token = gh::get_token();
    let client = gh::new_client(&token)?;

    if cli.list {
        commands::list(client, &cli.repo, &cli.include_drafts).await?;
    } else {
        commands::install(client, &cli.repo, &cli.include_drafts).await?;
    }

    Ok(())
}
