use crate::command::Command as _;
use crate::commands::Commands;
use anyhow::{Context, Error};
use log::{debug, error, warn};
use rumqttc::{AsyncClient, QoS};
use std::collections::HashMap;

pub struct CommandSubscriber<'a> {
    commands: &'a Commands,
    topic_to_command: HashMap<&'a str, CommandType>,
}

enum CommandType {
    Reboot,
}

impl<'a> CommandSubscriber<'a> {
    pub fn new(commands: &'a Commands) -> Self {
        let topic_to_command = [(commands.reboot_command.topic(), CommandType::Reboot)].into();
        Self {
            commands,
            topic_to_command,
        }
    }

    pub async fn subscribe_to_commands(&self, client: &AsyncClient) -> Result<(), Error> {
        for &topic in self.topic_to_command.keys() {
            debug!("Subscribing to {topic}");
            client
                .subscribe(topic, QoS::AtLeastOnce)
                .await
                .with_context(|| format!("Failed to subscribe to {topic}"))?;
        }
        Ok(())
    }

    pub async fn handle_message(&self, topic: &str) {
        debug!("Received command message on {topic}");

        match self.topic_to_command.get(topic) {
            Some(&CommandType::Reboot) => {
                if let Err(e) = self.commands.reboot_command.execute().await {
                    error!("Failed to execute reboot command: {e}");
                }
            }
            None => warn!("Received message on unknown topic {topic}"),
        }
    }
}
