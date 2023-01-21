use crate::{gh, repo::Repo};
use anyhow::{anyhow, bail, Result};
use futures_util::StreamExt;
use indicatif::ProgressBar;
use octocrab::models::repos::Release;
use octocrab::Octocrab;
use reqwest::header::HeaderMap;
use std::cmp::min;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub async fn install(client: Octocrab, repo: &Repo) -> Result<()> {
    if env::consts::FAMILY != "unix" {
        // TODO: Support windows
        bail!("Non-unix systems are not supported.");
    }

    // Get the release
    let release = get_release(&client, repo).await?;
    // Download the release
    let exec_path = download_and_unpack(repo, &release).await?;
    if exec_path.is_none() {
        bail!("Failed to find a valid executable for '{}'", repo.full_name);
    } else {
        let link_dir = dirs::home_dir().unwrap().join(".local/bin");

        let exec_dir = match env::consts::OS {
            "macos" => link_dir.as_path(),
            "linux" => link_dir.as_path(),
            os => bail!("OS '{}' is not supported right now.", os),
        };

        let link_path = exec_dir.join(exec_path.as_ref().unwrap().file_name().unwrap());

        // Create link for executable
        match std::os::unix::fs::symlink(exec_path.as_ref().unwrap(), link_path) {
            Ok(_) => {}
            Err(err) => bail!("Failed to create symlink - {}", err),
        };
    }

    Ok(())
}

async fn get_release(client: &Octocrab, repo: &Repo) -> Result<Release> {
    let handler = client.repos(&repo.owner, &repo.name);

    let release: Release;
    if repo.tag == "latest" {
        // Pull latest tag and use it.
        let tags = handler.list_tags().per_page(1).send().await?;

        if tags.items.len() == 0 {
            bail!("No tags found");
        }

        release = handler.releases().get_by_tag(&tags.items[0].name).await?;
    } else {
        // Use specified tag.
        release = handler.releases().get_by_tag(&repo.tag).await?;
    }

    Ok(release)
}

pub fn unzip(file_path: &Path, target_dir: &Path) -> Result<()> {
    let out = Command::new("tar")
        .stderr(Stdio::inherit())
        .arg("-xf")
        .arg(file_path.to_str().unwrap())
        .arg("-C")
        .arg(target_dir.to_str().unwrap())
        .output()?;

    if !out.status.success() {
        bail!("Failed to unzip: {}", file_path.to_str().unwrap());
    }

    Ok(())
}

/// Downloads and unpacks all of a release's assets. Each asset is unpacked,
/// tested, and the valid executable path is returned.
async fn download_and_unpack(repo: &Repo, release: &Release) -> Result<Option<PathBuf>> {
    // ensure dir exists
    repo.create_dir()?;

    let client = reqwest::Client::new();
    let token = gh::get_token();

    let mut headers = HeaderMap::new();
    if token != "" {
        headers.insert(
            "Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
    }

    let target_dir = repo.dir().join("unpacked");
    fs::create_dir_all(&target_dir)?;

    let mut executable_path: Option<PathBuf> = None;

    for asset in release.assets.iter() {
        if !asset.name.ends_with(".tar.gz") {
            continue;
        }

        let file_path = repo.dir().join(&asset.name);

        download_with_progress(
            &client,
            &headers,
            &asset.url.to_string(),
            &asset.content_type,
            file_path.to_str().unwrap(),
        )
        .await?;

        unzip(&file_path, &target_dir)?;

        let exec_path = get_exec_path(&target_dir)?;
        if exec_path.is_none() {
            continue;
        } else {
            executable_path = exec_path.clone();
            break;
        }
    }

    Ok(executable_path)
}

fn architecture_matches(val: String) -> bool {
    match env::consts::ARCH {
        "x86_64" => val.contains("executable x86"),
        "aarch64" => val.contains("executable arm64"),
        _ => false,
    }
}

/// Tests each file in the supplied directory to see if it's executable, matches
/// the calling architecture, and returns if these pass.
fn get_exec_path(dir: &PathBuf) -> Result<Option<PathBuf>> {
    let mut result: Option<PathBuf> = None;

    for entry in fs::read_dir(dir)? {
        let file = entry?;

        let cmd = format!("[ -x {} ]", file.path().to_str().unwrap());
        let is_executable = Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .output()?
            .status
            .success();

        // Only allow executable file.
        if is_executable {
            // Check to see we can execute it on the OS.
            let exec_out = Command::new(file.path().to_str().unwrap()).output()?;

            if exec_out.status.success() {
                let file_out = Command::new("file")
                    .arg("-b")
                    .arg(file.path().to_str().unwrap())
                    .output()?;

                let val = String::from_utf8(file_out.stdout)?.to_lowercase();

                if architecture_matches(val) {
                    result = Some(file.path());
                    break;
                }
            }
        }
    }

    Ok(result)
}

async fn download_with_progress(
    client: &reqwest::Client,
    headers: &HeaderMap,
    url: &str,
    content_type: &str,
    file_path: &str,
) -> Result<()> {
    // Right now, stream_asset() is broken on Octocrab. We'll download
    // ourselves.
    let resp = client
        .get(url)
        .header("Accept", "application/octet-stream")
        .header("User-Agent", "technicallyjosh/ghi")
        .header("Content-Type", content_type)
        .headers(headers.to_owned())
        .send()
        .await?;

    let total_size = resp
        .content_length()
        .ok_or(anyhow!("failed to get content length for {}", url,))?;
    let pb = ProgressBar::new(total_size);

    let mut file = File::create(file_path)?;
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(anyhow!("failed to download file {}", url)))?;
        file.write_all(&chunk)
            .or(Err(anyhow!("failed to write to file {}", url)))?;

        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_and_clear();

    Ok(())
}
