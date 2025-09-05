use crate::command::Command;
use crate::commands::Commands;
use anyhow::{Context, Error};
use log::{debug, error, warn};
use rumqttc::{AsyncClient, QoS};
use std::collections::HashMap;

pub struct CommandSubscriber<'a> {
    topic_to_command: HashMap<&'a str, &'a dyn Command>,
}

impl<'a> CommandSubscriber<'a> {
    pub fn new(commands: &'a Commands) -> Self {
        let topic_to_command = [
            (
                commands.reboot_command.topic(),
                &commands.reboot_command as &dyn Command,
            ),
            (
                commands.suspend_command.topic(),
                &commands.suspend_command as &dyn Command,
            ),
        ]
        .into();
        Self { topic_to_command }
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
            Some(command) => {
                if let Err(e) = command.execute().await {
                    let (_, id) = command.topic().rsplit_once('/').unwrap();
                    error!("Failed to execute command {id}: {e}");
                }
            }
            None => warn!("Received message on unknown topic {topic}"),
        }
    }
}
