use std::path::PathBuf;

use axum::{extract::Path, routing::get, Json, Router};
use indexmap::IndexMap;
use ring::digest::{digest, SHA256};
use serde_json::{json, Value};
use tokio::{fs, io::AsyncReadExt as _};
use walkdir::WalkDir;

use crate::{api::errors::internal_and_log, ApiError, ApiResult, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(versions))
        .route("/v1", get(v1))
        .route("/v2", get(v2))
        .route("/*path", get(download))
}

async fn versions() -> Json<Value> {
    Json(json!(["v1", "v2"]))
}

async fn v1() -> ApiResult<Json<IndexMap<String, Value>>> {
    let map = index_assets("v1").await.map_err(|err| internal_and_log(err))?;
    Ok(Json(map))
}

async fn v2() -> ApiResult<Json<IndexMap<String, Value>>> {
    let map = index_assets("v2").await.map_err(|err| internal_and_log(err))?;
    Ok(Json(map))
}

async fn download(Path(path): Path<String>) -> ApiResult<Vec<u8>> {
    let mut file = if let Ok(file) = fs::File::open(format!("assets/{path}")).await {
        file
    } else {
        return Err(ApiError::NotFound)
    };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await.map_err(|err| internal_and_log(err))?;
    Ok(buffer)
}

// non web

async fn index_assets(version: &str) -> anyhow::Result<IndexMap<String, Value>> {
    let mut map = IndexMap::new();
    let version_path = PathBuf::from("assets/").join(version);

    for entry in WalkDir::new(version_path.clone()).into_iter().filter_map(|e| e.ok()) {
        let data = match fs::read(entry.path()).await {
            Ok(d) => d,
            Err(_) => continue
        };

        let path: String;

        if cfg!(windows) {
            path = entry.path().strip_prefix(version_path.clone())?.to_string_lossy().to_string().replace("\\", "/");
        } else {
            path = entry.path().strip_prefix(version_path.clone())?.to_string_lossy().to_string();
        }

        map.insert(path, Value::from(hex::encode(digest(&SHA256, &data).as_ref())));
    }

    Ok(map)
}