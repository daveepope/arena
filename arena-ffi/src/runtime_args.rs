use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RuntimeArgConfig {
    pub name: String,
    pub value: String,
}
