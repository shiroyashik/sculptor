use serde::Deserialize;

#[derive(Deserialize)]
pub struct Id {
    pub username: String,
}

#[derive(Deserialize)]
pub struct Verify {
    pub id: String,
}