use arena_kafka::TopicCreator;

const UNREACHABLE_BOOTSTRAP: &str = "127.0.0.1:1";

#[test]
fn clear_messages_unreachable_bootstrap_returns_err() {
    let result = TopicCreator::clear_messages(UNREACHABLE_BOOTSTRAP, "topic-creator-unreachable");
    assert!(result.is_err());
}

#[test]
fn clear_messages_invalid_bootstrap_string_returns_err() {
    let result = TopicCreator::clear_messages("", "topic-creator-invalid");
    assert!(result.is_err());
}
