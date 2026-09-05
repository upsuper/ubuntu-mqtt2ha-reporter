use crate::commands::reboot::RebootCommand;
use crate::commands::suspend::SuspendCommand;
use crate::commands::update::UpdateCommand;
use tokio::sync::mpsc;

mod reboot;
mod suspend;
mod update;

pub struct Commands {
    pub update_command: UpdateCommand,
    pub reboot_command: RebootCommand,
    pub suspend_command: SuspendCommand,
}

pub fn create_commands(topic_base: &str, force_update: mpsc::Sender<()>) -> Commands {
    let topic_base = format!("{topic_base}/command");
    let update_command = UpdateCommand::new(&topic_base, force_update);
    let reboot_command = RebootCommand::new(&topic_base);
    let suspend_command = SuspendCommand::new(&topic_base);
    Commands {
        update_command,
        reboot_command,
        suspend_command,
    }
}
