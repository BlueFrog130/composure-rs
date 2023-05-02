use std::collections::HashMap;

pub mod auth;
pub mod models;

use models::{ApplicationCommandInteraction, InteractionResponse};

pub enum Error {
    CommandAlreadyInserted,
}

pub type Result<T> = std::result::Result<T, Error>;

pub type CommandMap = HashMap<&'static str, CommandHandler>;

pub type CommandHandler = fn(command: ApplicationCommandInteraction) -> InteractionResponse;

pub trait InteractionBot {
    fn get_commands(&mut self) -> &mut CommandMap;

    fn handle_command(&mut self, command: &'static str, handler: CommandHandler) -> &Self {
        self.get_commands().insert(command, handler);

        self
    }
}
