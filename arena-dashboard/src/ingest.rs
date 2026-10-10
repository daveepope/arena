use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct IngestFault {
    pub id: String,
    pub subject: String,
    pub message: String,
    pub at: String,
    #[serde(default)]
    pub faults: Vec<IngestFault>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestSubject {
    pub id: String,
    pub state: String,
    #[serde(default)]
    pub faults: Vec<IngestFault>,
    #[serde(default)]
    pub children: Vec<IngestSubject>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestArenaState {
    pub id: String,
    pub state: String,
    pub at: String,
    #[serde(default)]
    pub dependencies: Vec<IngestSubject>,
    #[serde(default)]
    pub components: Vec<IngestSubject>,
    #[serde(default)]
    pub faults: Vec<IngestFault>,
}

pub fn parse(body: &str) -> Result<IngestArenaState, serde_json::Error> {
    serde_json::from_str(body)
}
