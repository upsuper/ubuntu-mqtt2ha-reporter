use crate::command::{Command, CommandDiscovery, CommandDiscoveryInit};
use crate::ha::values::EntityCategory;
use anyhow::Error;
use async_trait::async_trait;
use log::info;
use tokio::sync::mpsc;

const ID: &str = "update";

pub struct UpdateCommand {
    topic: Box<str>,
    tx: mpsc::Sender<()>,
}

impl UpdateCommand {
    pub fn new(topic_base: &str, tx: mpsc::Sender<()>) -> Self {
        Self {
            topic: format!("{topic_base}/{ID}").into(),
            tx,
        }
    }
}

#[async_trait]
impl Command for UpdateCommand {
    fn topic(&self) -> &str {
        &self.topic
    }

    fn discovery_data(&self) -> Vec<CommandDiscovery<'_>> {
        vec![
            CommandDiscovery::new(CommandDiscoveryInit {
                id: ID,
                name: "Update Sensors",
                icon: "mdi:update",
            })
            .with_entity_category(EntityCategory::Config),
        ]
    }

    async fn execute(&self) -> Result<(), Error> {
        info!("Triggering sensor update");
        self.tx.send(()).await?;
        Ok(())
    }
}
