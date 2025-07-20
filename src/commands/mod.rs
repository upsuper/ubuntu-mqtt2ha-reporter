use crate::commands::reboot::RebootCommand;

mod reboot;

pub struct Commands {
    pub reboot_command: RebootCommand,
}

pub fn create_commands(topic_base: &str) -> Commands {
    let topic_base = format!("{topic_base}/command");
    let reboot_command = RebootCommand::new(&topic_base);
    Commands { reboot_command }
}
