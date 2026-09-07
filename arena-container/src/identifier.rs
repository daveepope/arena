use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

const SUFFIX_LEN: usize = 6;
const MODULE_PREFIX: &str = "arena-";
const BASE36: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

pub fn build(module: &str, name: &str) -> String {
    if is_already_built(name) {
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

pub fn resolve_container_name(identifier: &str, override_name: Option<&str>) -> String {
    match override_name {
        Some(name) => name.to_string(),
        None => sanitize_for_container(identifier),
    }
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

fn is_already_built(name: &str) -> bool {
    let Some(rest) = name.strip_prefix(MODULE_PREFIX) else {
        return false;
    };
    let Some(suffix) = rest.rsplit('-').next() else {
        return false;
    };
    suffix.len() == SUFFIX_LEN
        && suffix
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}
