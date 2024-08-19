use anyhow::anyhow;
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::{FIGURA_RELEASES_URL, TIMEOUT, USER_AGENT};

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