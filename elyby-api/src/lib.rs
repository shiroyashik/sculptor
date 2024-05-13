use anyhow::{anyhow, Result};
use log::debug;

pub async fn has_joined(server_id: &str, username: &str) -> Result<bool> {
    let client = reqwest::Client::new();
    let res = client.get(
        format!("http://minecraft.ely.by/session/hasJoined?serverId={server_id}&username={username}")).send().await?;
    debug!("{res:?}");
    match res.status().as_u16() {
        200 => Ok(true),
        401 => Ok(false),
        _ => Err(anyhow!("Unknown code: {}", res.status().as_u16()))
    }
}

#[tokio::test]
async fn test_has_joined() {
    let result = has_joined("0f8fef917f1f62b963804d822b67fe6f59aad7d", "test").await.unwrap();
    assert_eq!(result, false)
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