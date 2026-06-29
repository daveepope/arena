use arbiter::{extract, Event, Features};

fn user(prompt_chars: usize, probe_hits: usize) -> Event {
    Event {
        role_user: true,
        prompt_chars,
        probe_hits,
        ..Event::default()
    }
}

fn assistant_edit(edit_bytes: usize) -> Event {
    Event {
        role_user: false,
        edit_bytes,
        ..Event::default()
    }
}

fn assistant_read() -> Event {
    Event {
        role_user: false,
        read_ops: 1,
        ..Event::default()
    }
}

#[test]
fn extract_empty_returns_zeroed_features() {
    let features = extract(&[]);
    assert_eq!(features, Features::default());
    assert_eq!(features.spec_gap, 0.0);
}

#[test]
fn extract_counts_user_and_assistant_turns() {
    let events = [user(10, 0), assistant_edit(5), user(10, 0)];
    let features = extract(&events);
    assert_eq!(features.user_turns, 2);
    assert_eq!(features.assistant_turns, 1);
}

#[test]
fn extract_blind_accept_returns_large_spec_gap() {
    let events = [user(2, 0), assistant_edit(400)];
    let features = extract(&events);
    assert!(features.spec_gap > 100.0);
}

#[test]
fn extract_consecutive_assistant_returns_max_autonomy() {
    let events = [
        user(5, 0),
        assistant_edit(1),
        assistant_read(),
        assistant_edit(1),
        user(5, 0),
        assistant_edit(1),
    ];
    let features = extract(&events);
    assert_eq!(features.max_autonomy_run, 3);
    assert_eq!(features.autonomy_streak, 1);
}

#[test]
fn extract_resets_autonomy_after_user_turn() {
    let events = [
        assistant_edit(1),
        assistant_edit(1),
        user(5, 0),
        assistant_edit(1),
    ];
    let features = extract(&events);
    assert_eq!(features.max_autonomy_run, 2);
    assert_eq!(features.autonomy_streak, 1);
}

#[test]
fn extract_ends_on_user_turn_zeros_autonomy_streak() {
    let events = [
        assistant_edit(1),
        assistant_edit(1),
        assistant_edit(1),
        user(5, 0),
    ];
    let features = extract(&events);
    assert_eq!(features.max_autonomy_run, 3);
    assert_eq!(features.autonomy_streak, 0);
}
