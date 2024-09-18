use std::{env::{self, var}, path::{self, PathBuf}};

use anyhow::anyhow;
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::{fs::{self, File}, io::{AsyncReadExt as _, AsyncWriteExt as _}};
use tracing::error;

use crate::{ASSETS_ENV, FIGURA_ASSETS_ZIP_URL, FIGURA_RELEASES_URL, TIMEOUT, USER_AGENT};

#[derive(Deserialize, Debug)]
struct Tag {
    name: String
}

async fn get_latest_version(repo: &str, current_version: Version) -> anyhow::Result<Option<String>> {
    let url = format!("https://api.github.com/repos/{repo}/tags");
    let client = Client::builder().timeout(TIMEOUT).user_agent(USER_AGENT).build().unwrap();
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let tags: Vec<Tag> = response.json().await?;
        let latest_tag = tags.iter()
            .filter_map(|tag| {
                if tag.name.starts_with('v') { // v#.#.#
                    Version::parse(&tag.name[1..]).ok()
                } else {
                    None
                }
            })
            .max();
        if let Some(latest_version) = latest_tag {
            if latest_version > current_version {
                Ok(Some(format!("Available new v{latest_version}")))
            } else {
                Ok(Some("Up to date".to_string()))
            }
        } else {
            Err(anyhow!("Can't find version tags!"))
        }
    } else {
        Err(anyhow!("Response status code: {}", response.status().as_u16()))
    }
}

pub async fn check_updates(repo: &str, current_version: &str) -> anyhow::Result<String> {
    let current_version = semver::Version::parse(&current_version)?;

    match get_latest_version(repo, current_version).await {
        Ok(d) => if let Some(text) = d {
            Ok(format!(" - {text}!"))
        } else {
            Ok(String::new())
        },
        Err(e) => {
            error!("Can't fetch updates: {e:?}");
            Ok(String::new())
        },
    }
}

// Figura

#[derive(Deserialize, Debug)]
struct Release {
    tag_name: String,
    prerelease: bool
}

pub async fn get_figura_versions() -> anyhow::Result<FiguraVersions> {
    let client = Client::builder().timeout(TIMEOUT).user_agent(USER_AGENT).build().unwrap();
    let response = client.get(FIGURA_RELEASES_URL).send().await?;

    let mut release_ver = Version::new(0, 0, 0);
    let mut prerelease_ver = Version::new(0, 0, 0);

    if response.status().is_success() {
        let multiple_releases: Vec<Release> = response.json().await?;
        for release in multiple_releases {
            let tag_ver = if let Ok(res) = Version::parse(&release.tag_name) { res } else {
                error!("Incorrect tag name! {release:?}");
                continue;
            };
            if release.prerelease {
                if tag_ver > prerelease_ver {
                    prerelease_ver = tag_ver
                }
            } else {
                if tag_ver > release_ver {
                    release_ver = tag_ver
                }
            }
        }
        if release_ver > prerelease_ver {
            prerelease_ver = release_ver.clone();
        }
        // Stop
        Ok(FiguraVersions { release: release_ver.to_string(), prerelease: prerelease_ver.to_string() })
    } else {
        Err(anyhow!("Response status code: {}", response.status().as_u16()))
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct FiguraVersions {
    pub release: String,
    pub prerelease: String
}

// Assets

#[derive(Deserialize, Debug)]
struct Commit {
    sha: String
}

pub fn get_path_to_assets_hash() -> PathBuf {
    path::PathBuf::from(&env::var(ASSETS_ENV).unwrap()).join("..").join("assets_last_commit")
}

pub async fn get_commit_sha(url: &str) -> anyhow::Result<String> {
    let client = Client::builder().timeout(TIMEOUT).user_agent(USER_AGENT).build().unwrap();
    let response: reqwest::Response = client.get(url).send().await?;
    let commit: Commit = response.json().await?;
    Ok(commit.sha)
}

pub async fn is_assets_outdated(last_sha: &str) -> anyhow::Result<bool> {
    let path = get_path_to_assets_hash();

    match File::open(path.clone()).await {
        Ok(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents).await?;
            if contents.lines().count() != 1 {
                // Lines count in file abnormal
                Ok(true)
            } else {
                if contents == last_sha {
                    Ok(false)
                } else {
                    // SHA in file mismatches with provided SHA
                    Ok(true)
                }
            }
        },
        Err(err) => if err.kind() == tokio::io::ErrorKind::NotFound {
            // Can't find file
            Ok(true)
        } else {
            anyhow::bail!("{:?}", err);
        }
    }
}

pub fn download_assets() -> anyhow::Result<()> {
    use std::{fs::{File, self}, io::Write as _};

    let assets_folder = var(ASSETS_ENV).unwrap();

    // Path to save the downloaded ZIP file
    let zip_file_path = path::PathBuf::from(&assets_folder).join("..").join("assets.zip");

    // Download the ZIP file

    let response = reqwest::blocking::get(FIGURA_ASSETS_ZIP_URL)?;
    let bytes = response.bytes()?;

    // Save the downloaded ZIP file to disk
    let mut file = File::create(&zip_file_path)?;
    file.write_all(&bytes)?;

    // Open the downloaded ZIP file
    let file = File::open(&zip_file_path)?;

    let mut archive = zip::ZipArchive::new(file)?;
    let mut extraction_info = String::from("Extraction complete! More info:\n");
    let mut first_folder = String::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let zipoutpath = match file.enclosed_name() {
            Some(path) => path,
            None => continue,
        };

        // Folder name spoofing
        if i == 0 {
            if file.is_dir() {
                first_folder = zipoutpath.to_str().ok_or_else(|| anyhow::anyhow!("0 index doesn't contains path!"))?.to_string();
            } else {
                anyhow::bail!("0 index is not a folder!")
            }
        }
        let mut outpath = path::PathBuf::from(&assets_folder);
        outpath.push(zipoutpath.strip_prefix(first_folder.clone())?);
        // Spoof end

        {
            let comment = file.comment();
            if !comment.is_empty() {
                extraction_info.push_str(&format!("File {i} comment: {comment}\n"));
            }
        }
        if file.is_dir() {
            extraction_info.push_str(&format!("Dir  {} extracted to \"{}\"\n", i, outpath.display()));
            fs::create_dir_all(&outpath)?;
        } else {
            extraction_info.push_str(&format!(
                "File {} extracted to \"{}\" ({} bytes)\n",
                i,
                outpath.display(),
                file.size()
            ));
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    extraction_info.pop(); // Removes \n from end
    tracing::debug!("{extraction_info}");
    Ok(())
}

pub async fn write_sha_to_file(sha: &str) -> anyhow::Result<()> {
    let path = get_path_to_assets_hash();

    let mut file = File::create(path).await?;
    file.write_all(sha.as_bytes()).await?;
    file.flush().await?;
    Ok(())
}

pub async fn remove_assets() {
    fs::remove_dir_all(&var(ASSETS_ENV).unwrap()).await.unwrap_or_else(|err| tracing::debug!("Assets dir remove failed due {err:?}"));
    fs::remove_file(get_path_to_assets_hash()).await.unwrap_or_else(|err| tracing::debug!("Assets hash file remove failed due {err:?}"));
}