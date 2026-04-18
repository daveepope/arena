use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RuntimeArgConfig {
    pub name: String,
    pub value: String,
}
