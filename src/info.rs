use axum::Json;
use serde_json::{json, Value};

pub async fn version() -> Json<Value> {
    Json(json!({
        "release": "0.1.4",
        "prerelease": "0.1.4"
    }))
}

pub async fn limits() -> Json<Value> {
    Json(json!({
        "rate": {
          "pingSize": 1024,
          "pingRate": 32,
          "equip": 1,
          "download": 50,
          "upload": 1
        },
        "limits": {
          "maxAvatarSize": 100000,
          "maxAvatars": 10,
          "allowedBadges": {
            "special": [0,0,0,0,0,0],
            "pride": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
          }
        }
    }))
}
