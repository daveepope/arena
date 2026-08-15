use super::client::{connect_client, partition_client_for_existing};
use rskafka::client::error::{Error as ClientError, ProtocolError};
use rskafka::client::partition::OffsetAt;
use rskafka::client::Client;

const TOPIC_PARTITION_COUNT: i32 = 1;
const TOPIC_REPLICATION_FACTOR: i16 = 1;
const CREATE_TOPIC_TIMEOUT_MS: i32 = 500;
const DELETE_RECORDS_TIMEOUT_MS: i32 = 500;

pub struct TopicCreator;

impl TopicCreator {
    pub async fn create_topic(client: &Client, topic: &str) -> Result<(), String> {
        let controller = client
            .controller_client()
            .map_err(|e| format!("create kafka controller client failed: {e}"))?;

        match controller
            .create_topic(
                topic,
                TOPIC_PARTITION_COUNT,
                TOPIC_REPLICATION_FACTOR,
                CREATE_TOPIC_TIMEOUT_MS,
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(ClientError::ServerError {
                protocol_error: ProtocolError::TopicAlreadyExists,
                ..
            }) => Ok(()),
            Err(e) => Err(format!("kafka topic create failed: {e}")),
        }
    }

    pub async fn create_topic_on(bootstrap: &str, topic: &str) -> Result<(), String> {
        let client = connect_client(bootstrap).await?;
        Self::create_topic(&client, topic).await
    }

    pub async fn clear_messages(client: &Client, topic: &str) -> Result<(), String> {
        let partition = partition_client_for_existing(client, topic).await?;

        let latest = partition
            .get_offset(OffsetAt::Latest)
            .await
            .map_err(|e| format!("get kafka latest offset failed: {e}"))?;

        partition
            .delete_records(latest, DELETE_RECORDS_TIMEOUT_MS)
            .await
            .map_err(|e| format!("kafka delete_records failed: {e}"))?;

        Ok(())
    }
}
