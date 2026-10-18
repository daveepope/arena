use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CpuProfilePayload {
    pub component: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub folded: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ScopedPlaybookPayload {
    pub arena_id: String,
    pub playbook_id: String,
    pub kind: String,
    pub state: String,
    pub at: String,
    #[serde(default)]
    pub faults: Vec<IngestFault>,
}

pub fn parse_scoped_playbook(body: &str) -> Result<ScopedPlaybookPayload, serde_json::Error> {
    serde_json::from_str(body)
}
