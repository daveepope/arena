use serde::Serialize;

#[derive(Debug, Clone)]
pub enum HeaderPattern {
    EqualTo(String),
    Matching(String),
}

impl HeaderPattern {
    pub fn equal_to(value: impl Into<String>) -> Self {
        Self::EqualTo(value.into())
    }

    pub fn matching(regex: impl Into<String>) -> Self {
        Self::Matching(regex.into())
    }
}

impl Serialize for HeaderPattern {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Self::EqualTo(val) => map.serialize_entry("equalTo", val)?,
            Self::Matching(regex) => map.serialize_entry("matches", regex)?,
        }
        map.end()
    }
}
