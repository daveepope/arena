use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct CreateReadingResponse {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
}

#[derive(Deserialize)]
pub struct CalibrationResponse {
    pub valid: bool,
}

#[derive(Serialize)]
pub struct ReadingRow {
    pub id: i64,
    pub user_name: String,
    pub value: i32,
    pub comment: Option<String>,
}

#[derive(Serialize)]
pub struct ReadingCreatedEvent<'a> {
    pub id: i64,
    pub user_name: &'a str,
    pub value: i32,
    pub comment: &'a Option<String>,
}
