use std::collections::HashSet;
use std::net::TcpListener;
use std::ops::RangeInclusive;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortSearchStrategy {
    #[default]
    Random,
    Linear,
}

pub fn find_available_port(range: RangeInclusive<u16>, strategy: PortSearchStrategy) -> Option<u16> {
    let mut candidates: Vec<u16> = range.collect();
    if let PortSearchStrategy::Random = strategy {
        shuffle(&mut candidates);
    }

    let reserved = reserved_ports();
    let mut reserved = reserved.lock().unwrap_or_else(|e| e.into_inner());
    for port in candidates {
        if reserved.contains(&port) {
            continue;
        }
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            reserved.insert(port);
            return Some(port);
        }
    }
    None
}

fn reserved_ports() -> &'static Mutex<HashSet<u16>> {
    static RESERVED: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();
    RESERVED.get_or_init(|| Mutex::new(HashSet::new()))
}

fn shuffle(candidates: &mut [u16]) {
    let mut rng = Xorshift64Star::seeded_from_time_and_thread();
    for i in (1..candidates.len()).rev() {
        let j = (rng.next_u64() as usize) % (i + 1);
        candidates.swap(i, j);
    }
}

struct Xorshift64Star(u64);

impl Xorshift64Star {
    fn seeded_from_time_and_thread() -> Self {
        Self(splitmix64(seed_material()).max(1))
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

fn seed_material() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    nanos ^ thread_id_hash()
}

fn thread_id_hash() -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::thread::current().id().hash(&mut hasher);
    hasher.finish()
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
