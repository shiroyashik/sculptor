use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::{utils::get_motd, AppState};

pub async fn version() -> Json<Value> {
    Json(json!({
        "release": "0.1.4",
        "prerelease": "0.1.4"
    }))
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
