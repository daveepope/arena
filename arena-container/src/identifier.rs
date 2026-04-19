use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn build(module: &str, name: &str) -> String {
    if has_guid_suffix(name) {
        return name.to_string();
    }
    let guid = new_guid();
    let name = name.trim();
    if name.is_empty() {
        format!("{module} - {guid}")
    } else {
        format!("{module} - {name} - {guid}")
    }
}

pub fn sanitize_for_container(identifier: &str) -> String {
    let mut out = String::with_capacity(identifier.len());
    let mut last_dash = false;
    for c in identifier.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn new_guid() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let pid = std::process::id() as u64;
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);

    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (nanos >> 32) as u32,
        (nanos >> 16) as u16,
        nanos as u16,
        pid as u16,
        seq & 0x0000_FFFF_FFFF_FFFF,
    )
}

fn has_guid_suffix(name: &str) -> bool {
    let Some(last) = name.rsplit(' ').next() else {
        return false;
    };
    if last.len() != 36 {
        return false;
    }
    let parts: Vec<&str> = last.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    let expected = [8usize, 4, 4, 4, 12];
    for (p, len) in parts.iter().zip(expected.iter()) {
        if p.len() != *len {
            return false;
        }
        if !p.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_module_name_guid_when_name_given() {
        let id = build("arena-http", "calibration service");
        assert!(id.starts_with("arena-http - calibration service - "));
        assert_eq!(id.split(" - ").count(), 3);
    }

    #[test]
    fn builds_module_guid_when_name_empty() {
        let id = build("arena-mssql", "");
        assert!(id.starts_with("arena-mssql - "));
        assert_eq!(id.split(" - ").count(), 2);
    }

    #[test]
    fn treats_whitespace_only_name_as_absent() {
        let id = build("arena-kafka", "   ");
        assert_eq!(id.split(" - ").count(), 2);
    }

    #[test]
    fn two_calls_produce_different_guids() {
        let a = build("arena-http", "x");
        let b = build("arena-http", "x");
        assert_ne!(a, b);
    }

    #[test]
    fn is_idempotent_when_identifier_already_built() {
        let once = build("arena-http", "calibration");
        let twice = build("arena-http", &once);
        assert_eq!(once, twice);
    }

    #[test]
    fn sanitizes_spaces_and_runs_of_non_alphanumerics() {
        let id = "arena-mssql - example validation - 18a79e95-bdba-7035-85ba-000000000000";
        let name = sanitize_for_container(id);
        assert_eq!(
            name,
            "arena-mssql-example-validation-18a79e95-bdba-7035-85ba-000000000000"
        );
        assert!(!name.contains(' '));
        assert!(!name.contains("--"));
    }
}
