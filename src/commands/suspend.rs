use crate::command::{Command, CommandDiscovery, CommandDiscoveryInit};
use anyhow::{Context, Error, anyhow};
use log::info;
use std::borrow::Cow;
use tokio::process;

const ID: &str = "suspend";

pub struct SuspendCommand {
    topic: Cow<'static, str>,
}

impl SuspendCommand {
    pub fn new(topic_base: &str) -> Self {
        Self {
            topic: format!("{topic_base}/{ID}").into(),
        }
    }
}

impl Command for SuspendCommand {
    fn topic(&self) -> &str {
        &self.topic
    }

    fn discovery_data(&self) -> Vec<CommandDiscovery<'_>> {
        vec![
            CommandDiscovery::new(CommandDiscoveryInit {
                id: ID,
                name: "Suspend System",
                icon: "mdi:sleep",
            })
            .with_device_class("restart"),
        ]
    }

    async fn execute(&self) -> Result<(), Error> {
        info!("Executing suspend command");

        let output = process::Command::new("sudo")
            .args(["-n", "/usr/bin/systemctl", "suspend"])
            .output()
            .await
            .context("Failed to execute suspend command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Suspend command failed: {}", stderr));
        }
        Ok(())
    }
}
