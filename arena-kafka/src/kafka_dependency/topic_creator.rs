use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::metadata::MetadataTopic;
use rdkafka::topic_partition_list::{Offset, TopicPartitionList};
use std::time::Duration;

pub struct TopicCreator;

impl TopicCreator {
    pub fn create_topic(bootstrap: &str, topic: &str) -> Result<(), String> {
        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set("log_level", "0")
            .create()
            .map_err(|e| format!("create kafka admin client failed: {e}"))?;

        let new_topic = NewTopic::new(topic, 1, TopicReplication::Fixed(1));
        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_millis(1000)));

        match futures::executor::block_on(admin.create_topics([&new_topic], &opts)) {
            Ok(results) => {
                for r in results {
                    if let Err((_t, e)) = r {
                        if e.to_string().to_lowercase().contains("already exists") {
                            return Ok(());
                        }
                        return Err(format!("kafka topic create failed: {e}"));
                    }
                }
                Ok(())
            }
            Err(err) => Err(format!("kafka topic create request failed: {err}")),
        }
    }

    pub fn clear_messages(bootstrap: &str, topic: &str) -> Result<(), String> {
        let consumer: BaseConsumer = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set("log_level", "0")
            .create()
            .map_err(|e| format!("create kafka consumer for metadata failed: {e}"))?;

        let metadata = consumer
            .fetch_metadata(Some(topic), Duration::from_secs(5))
            .map_err(|e| format!("fetch topic metadata failed: {e}"))?;

        let topic_meta = metadata
            .topics()
            .iter()
            .find(|t: &&MetadataTopic| t.name() == topic)
            .ok_or_else(|| format!("topic {topic} not found"))?;

        if topic_meta.error().is_some() {
            return Err(format!(
                "topic {topic} metadata error: {:?}",
                topic_meta.error()
            ));
        }

        let mut offsets = TopicPartitionList::new();
        for p in topic_meta.partitions() {
            let partition_id: i32 = p.id();
            offsets
                .add_partition_offset(topic, partition_id, Offset::End)
                .map_err(|e| format!("add partition to delete list failed: {e}"))?;
        }

        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set("log_level", "0")
            .create()
            .map_err(|e| format!("create kafka admin client failed: {e}"))?;

        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(5)));
        futures::executor::block_on(admin.delete_records(&offsets, &opts))
            .map_err(|e| format!("kafka delete_records failed: {e}"))?;

        Ok(())
    }
}
