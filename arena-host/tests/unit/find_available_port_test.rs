use std::net::TcpListener;

use arena_host::find_available_port::{find_available_port, PortSearchStrategy};

#[test]
fn find_available_port_random_strategy_returns_port_within_range() {
    let port = find_available_port(21000..=21099, PortSearchStrategy::Random)
        .expect("expected a free port");
    assert!((21000..=21099).contains(&port));
}

#[test]
fn find_available_port_linear_strategy_returns_lowest_free_port() {
    let range = 21200u16..=21209u16;
    let held: Vec<TcpListener> = (21200..21205)
        .map(|p| TcpListener::bind(("127.0.0.1", p)).expect("bind held port"))
        .collect();

    let port = find_available_port(range, PortSearchStrategy::Linear).expect("expected a free port");
    assert_eq!(port, 21205);

    drop(held);
}

#[test]
fn find_available_port_occupied_port_is_skipped_not_returned() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral");
    let occupied_port = listener.local_addr().unwrap().port();

    let result = find_available_port(occupied_port..=occupied_port, PortSearchStrategy::Linear);
    assert_eq!(result, None);

    drop(listener);
}

#[test]
fn find_available_port_two_sequential_calls_never_return_same_port() {
    let range = 21300u16..=21309u16;
    let first =
        find_available_port(range.clone(), PortSearchStrategy::Linear).expect("first port");
    let second = find_available_port(range, PortSearchStrategy::Linear).expect("second port");
    assert_ne!(first, second);
}

#[test]
fn find_available_port_concurrent_calls_never_collide() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let range = 21400u16..=21449u16;
    let results = Arc::new(Mutex::new(Vec::new()));
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let results = Arc::clone(&results);
            let range = range.clone();
            thread::spawn(move || {
                if let Some(p) = find_available_port(range, PortSearchStrategy::Random) {
                    results.lock().unwrap().push(p);
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }

    let mut ports = results.lock().unwrap().clone();
    let before = ports.len();
    ports.sort_unstable();
    ports.dedup();
    assert_eq!(before, ports.len(), "no two threads should receive the same port");
}

#[test]
fn find_available_port_inverted_range_returns_none() {
    assert_eq!(find_available_port(600..=500, PortSearchStrategy::Linear), None);
}

#[test]
fn find_available_port_range_reaches_u16_max() {
    let held = TcpListener::bind(("127.0.0.1", 65534)).expect("bind held port");

    let port = find_available_port(65534..=u16::MAX, PortSearchStrategy::Linear)
        .expect("expected port 65535 to be reachable");
    assert_eq!(port, u16::MAX);

    drop(held);
}

#[test]
#[should_panic(expected = "no available port found in range")]
fn find_available_port_fully_occupied_range_expect_panics_with_clear_message() {
    let range = 21500u16..=21501u16;
    let held: Vec<TcpListener> = range
        .clone()
        .map(|p| TcpListener::bind(("127.0.0.1", p)).expect("bind held port"))
        .collect();

    let start = *range.start();
    let end = *range.end();
    let result = find_available_port(range, PortSearchStrategy::Linear);
    drop(held);
    result.unwrap_or_else(|| panic!("no available port found in range {start}..={end}"));
}
