use axum::{extract::State, Json};
use serde_json::{json, Value};
use tracing::error;

use crate::{
    utils::{get_figura_versions, get_motd, FiguraVersions}, AppState, FIGURA_DEFAULT_VERSION
};

pub async fn version(State(state): State<AppState>) -> Json<FiguraVersions> {
    let res = state.figura_versions.read().await.clone();
    if let Some(res) = res {
        Json(res)
    } else {
        let actual = get_figura_versions().await;
        if let Ok(res) = actual {
            let mut stored = state.figura_versions.write().await;
            *stored = Some(res);
            return Json(stored.clone().unwrap())
        } else {
            error!("get_figura_versions: {:?}", actual.unwrap_err());
        }
        Json(FiguraVersions {
            release: FIGURA_DEFAULT_VERSION.to_string(),
            prerelease: FIGURA_DEFAULT_VERSION.to_string()
        })
    }
}

pub async fn motd(State(state): State<AppState>) -> String {
    serde_json::to_string_pretty(&get_motd(state).await).unwrap()
}

pub async fn limits(State(state): State<AppState>) -> Json<Value> {
    let state = &state.config.read().await.limitations;
    Json(json!({
        "rate": {
            "pingSize": 1024,
            "pingRate": 32,
            "equip": 1,
            "download": 50,
            "upload": 1
        },
        "limits": {
            "maxAvatarSize": state.max_avatar_size,
            "maxAvatars": state.max_avatars,
            "allowedBadges": {
                "special": [0,0,0,0,0,0],
                "pride": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
            }
        }
    }))
}
