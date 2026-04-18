use async_trait::async_trait;

#[async_trait]
pub trait Playbook: Send {
    type Active: Send;

    async fn run(self) -> Self::Active;
}
