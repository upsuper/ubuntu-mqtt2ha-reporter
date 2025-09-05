use crate::command::{Command, CommandDiscovery, CommandDiscoveryInit};
use anyhow::{Context, Error, anyhow};
use async_trait::async_trait;
use log::info;
use tokio::process;

const ID: &str = "reboot";

pub struct RebootCommand {
    topic: Box<str>,
}

impl RebootCommand {
    pub fn new(topic_base: &str) -> Self {
        Self {
            topic: format!("{topic_base}/{ID}").into(),
        }
    }
}

#[async_trait]
impl Command for RebootCommand {
    fn topic(&self) -> &str {
        &self.topic
    }

    fn discovery_data(&self) -> Vec<CommandDiscovery<'_>> {
        vec![
            CommandDiscovery::new(CommandDiscoveryInit {
                id: ID,
                name: "Reboot System",
                icon: "mdi:restart",
            })
            .with_device_class("restart"),
        ]
    }

    async fn execute(&self) -> Result<(), Error> {
        info!("Executing reboot command");

        let output = process::Command::new("sudo")
            .args(["-n", "/usr/bin/systemctl", "reboot"])
            .output()
            .await
            .context("Failed to execute reboot command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Reboot command failed: {}", stderr));
        }
        Ok(())
    }
}
