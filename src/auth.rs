use axum::{extract::{Query, State}, routing::get, Router, debug_handler};
use serde::Deserialize;
use ring::digest::{self, digest};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/id", get(id))
        .route("/verify", get(verify))
}

#[derive(Deserialize)]
struct Id {username: String}

#[debug_handler]
async fn id(
    Query(query): Query<Id>,
    State(state): State<AppState>,
) -> String {
    let server_id = bytes_into_string(&digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, &rand()).as_ref()[0 .. 20]);
    let state = state.pending.lock().expect("Mutex poisoned!");
    state.insert(server_id.clone(), query.username);
    server_id
}

#[derive(Deserialize)]
struct Verify {id: String}

#[debug_handler]
async fn verify(
    Query(query): Query<Verify>,
    State(state): State<AppState>,
) -> String {
    let server_id = query.id.clone();
    let username = state.pending.lock().expect("Mutex poisoned!").remove(&server_id).unwrap().1;
    if !elyby_api::has_joined(&server_id, &username).await.unwrap() {
        return String::from("failed to verify")
    }
    let authenticated = state.authenticated.lock().expect("Mutex poisoned!");
    authenticated.insert(server_id.clone(), username);
    format!("{server_id}")
}

fn rand() -> [u8; 50] {
    use rand::{Rng, thread_rng};
    let mut rng = thread_rng();
    let distr = rand::distributions::Uniform::new_inclusive(0, 255);
    let mut nums: [u8; 50] = [0u8; 50];
    for x in &mut nums {
        *x = rng.sample(distr);
    }
    nums
}

pub fn bytes_into_string(code: &[u8]) -> String {
    use std::fmt::Write;
    let mut result = String::new();
    for byte in code {
        write!(result, "{:02x}", byte).unwrap();
    }
    result
}