use anyhow::{anyhow, Result};
use log::debug;
use serde_json::Value;
use uuid::Uuid;

pub async fn has_joined(server_id: &str, username: &str) -> Result<Option<Uuid>> {
    let client = reqwest::Client::new();
    let res = client.get(
        format!("http://minecraft.ely.by/session/hasJoined?serverId={server_id}&username={username}")).send().await?;
    debug!("{res:?}");
    match res.status().as_u16() {
        200 => {
            let json = serde_json::from_str::<Value>(&res.text().await?)?;
            let uuid = Uuid::parse_str(json["id"].as_str().unwrap())?;
            Ok(Some(uuid))
        },
        401 => Ok(None),
        _ => Err(anyhow!("Unknown code: {}", res.status().as_u16()))
    }
}

#[tokio::test]
async fn test_has_joined() {
    let result = has_joined("0f8fef917f1f62b963804d822b67fe6f59aad7d", "test").await.unwrap();
    assert_eq!(result, None)
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }