use std::collections::HashMap;

pub mod auth;
pub mod models;

use models::{ApplicationCommandInteraction, InteractionResponse};

pub type CommandMap = HashMap<&'static str, CommandHandler>;

pub type CommandHandler = fn(command: ApplicationCommandInteraction) -> InteractionResponse;

pub trait InteractionBot {}
