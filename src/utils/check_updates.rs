use anyhow::anyhow;
use reqwest::Client;
use semver::Version;
use serde::Deserialize;
use tracing::error;

#[derive(Deserialize, Debug)]
struct Tag {
    name: String
}

async fn get_latest_version(repo: &str, current_version: Version) -> anyhow::Result<Option<String>> {
    let url = format!("https://api.github.com/repos/{repo}/tags");
    let client = Client::new();
    let response = client.get(&url).header("User-Agent", "reqwest").send().await?;

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

