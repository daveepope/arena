use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateReadingRequest {
    pub user_name: String,
    pub value: i32,
    pub comment: Option<String>,
}
