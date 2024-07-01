use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub(super) struct UserUuid {
    pub uuid: Option<Uuid>,
}