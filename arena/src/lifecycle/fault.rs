use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Subject {
    Arena,
    Dependency,
    Component,
    Playbook,
}

impl Subject {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Arena => "arena",
            Self::Dependency => "dependency",
            Self::Component => "component",
            Self::Playbook => "playbook",
        }
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Fault {
    pub id: String,
    pub subject: Subject,
    pub message: String,
    #[serde(serialize_with = "serialize_timestamp")]
    pub at: DateTime<Utc>,
    pub faults: Vec<Fault>,
}

impl Fault {
    pub fn new(id: impl Into<String>, subject: Subject, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            subject,
            message: message.into(),
            at: Utc::now(),
            faults: Vec::new(),
        }
    }

    pub fn arena(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, Subject::Arena, message)
    }

    pub fn dependency(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, Subject::Dependency, message)
    }

    pub fn component(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, Subject::Component, message)
    }

    pub fn playbook(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, Subject::Playbook, message)
    }

    pub fn from_panic(
        id: impl Into<String>,
        subject: Subject,
        payload: &(dyn std::any::Any + Send),
    ) -> Self {
        Self::new(id, subject, panic_message(payload))
    }

    pub fn caused_by(mut self, fault: Fault) -> Self {
        self.faults.push(fault);
        self
    }

    pub fn caused_by_all(mut self, faults: impl IntoIterator<Item = Fault>) -> Self {
        self.faults.extend(faults);
        self
    }

    pub fn timestamp(&self) -> String {
        self.at.to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    pub fn flatten(&self) -> Vec<&Fault> {
        let mut out = vec![self];
        for nested in &self.faults {
            out.extend(nested.flatten());
        }
        out
    }
}

impl Fault {
    pub(crate) fn render(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        write!(
            f,
            "[{}] {} '{}': {}",
            self.timestamp(),
            self.subject,
            self.id,
            self.message
        )?;
        let indent = "  ".repeat(depth + 1);
        for nested in &self.faults {
            write!(f, "\n{indent}caused by ")?;
            nested.render(f, depth + 1)?;
        }
        Ok(())
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.render(f, 0)
    }
}

impl std::error::Error for Fault {}

pub(crate) fn serialize_timestamp<S>(at: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&at.to_rfc3339_opts(SecondsFormat::Millis, true))
}

pub fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_string()
    }
}
