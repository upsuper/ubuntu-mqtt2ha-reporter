use crate::commands::reboot::RebootCommand;
use crate::commands::suspend::SuspendCommand;

mod reboot;
mod suspend;

pub struct Commands {
    pub reboot_command: RebootCommand,
    pub suspend_command: SuspendCommand,
}

pub fn create_commands(topic_base: &str) -> Commands {
    let topic_base = format!("{topic_base}/command");
    let reboot_command = RebootCommand::new(&topic_base);
    let suspend_command = SuspendCommand::new(&topic_base);
    Commands {
        reboot_command,
        suspend_command,
    }
}
