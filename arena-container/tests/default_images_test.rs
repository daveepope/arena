use arena_container::default_images::{ALL, HTTP, KAFKA_APACHE, KAFKA_CONFLUENT, LOCALSTACK, MSSQL, POSTGRES};

#[test]
fn default_images_all_entries_have_distinct_ids() {
    let mut ids = ALL.iter().map(|entry| entry.id).collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), ALL.len());
}

#[test]
fn default_images_postgres_matches_toml_default() {
    assert_eq!(POSTGRES.id, "postgres");
    assert_eq!(POSTGRES.image, "postgres");
    assert_eq!(POSTGRES.tag, "18-bookworm");
}

#[test]
fn default_images_builder_ids_match_toml_defaults() {
    assert_eq!(HTTP.image, "wiremock/wiremock");
    assert_eq!(HTTP.tag, "3.13.2-alpine");
    assert_eq!(KAFKA_APACHE.image, "apache/kafka");
    assert_eq!(KAFKA_APACHE.tag, "4.3.1");
    assert_eq!(KAFKA_CONFLUENT.image, "confluentinc/cp-kafka");
    assert_eq!(KAFKA_CONFLUENT.tag, "8.2.2");
    assert_eq!(MSSQL.image, "mcr.microsoft.com/mssql/server");
    assert_eq!(MSSQL.tag, "2025-CU7-ubuntu-24.04");
    assert_eq!(LOCALSTACK.image, "localstack/localstack");
    assert_eq!(LOCALSTACK.tag, "2026.06.3");
}
