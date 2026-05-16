use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

const SUFFIX_LEN: usize = 6;
const BASE36: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

pub fn build(module: &str, name: &str) -> String {
    if has_suffix(name) {
        return name.to_string();
    }
    let slug = slugify(name);
    let suffix = new_suffix();
    if slug.is_empty() {
        format!("{module}-{suffix}")
    } else {
        format!("{module}-{slug}-{suffix}")
    }
}

pub fn sanitize_for_container(identifier: &str) -> String {
    slugify(identifier)
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for c in s.chars() {
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

fn new_suffix() -> String {
    static SEED: OnceLock<u64> = OnceLock::new();
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let seed = *SEED.get_or_init(|| {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        nanos ^ (std::process::id() as u64).rotate_left(32)
    });
    let n = seed.wrapping_add(COUNTER.fetch_add(1, Ordering::Relaxed));
    to_base36(n)
}

fn to_base36(mut n: u64) -> String {
    let mut buf = [b'0'; SUFFIX_LEN];
    for slot in buf.iter_mut().rev() {
        *slot = BASE36[(n % 36) as usize];
        n /= 36;
    }
    String::from_utf8(buf.to_vec()).expect("base36 digits are ascii")
}

fn has_suffix(name: &str) -> bool {
    let Some(last) = name.rsplit('-').next() else {
        return false;
    };
    last.len() == SUFFIX_LEN
        && last
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_module_name_suffix_when_name_given() {
        let id = build("arena-http", "calibration service");
        assert!(id.starts_with("arena-http-calibration-service-"));
        assert!(!id.contains(' '));
        let suffix = id.rsplit('-').next().unwrap();
        assert_eq!(suffix.len(), SUFFIX_LEN);
        assert!(suffix
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    #[test]
    fn builds_module_suffix_when_name_empty() {
        let id = build("arena-mssql", "");
        assert!(id.starts_with("arena-mssql-"));
        assert!(!id.contains(' '));
    }

    #[test]
    fn treats_whitespace_only_name_as_absent() {
        let id = build("arena-kafka", "   ");
        assert!(id.starts_with("arena-kafka-"));
        assert_eq!(id.matches('-').count(), 2);
    }

    #[test]
    fn two_calls_produce_different_suffixes() {
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
    fn sanitize_is_noop_on_clean_identifier() {
        let id = "arena-mssql-example-validation-a1b2c3";
        assert_eq!(sanitize_for_container(id), id);
    }

    #[test]
    fn sanitize_collapses_spaces_and_non_alphanumerics() {
        assert_eq!(sanitize_for_container("Hello World!!"), "hello-world");
    }
}
