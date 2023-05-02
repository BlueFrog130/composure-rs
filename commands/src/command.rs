mod implementation;
mod model;

pub use implementation::*;
pub use model::*;

#[cfg(test)]
mod tests {
    use interaction_bot::models::{Snowflake, TypeField};

    use super::*;

    #[test]
    pub fn serialize_command() {
        let command = ApplicationCommand::new_chat_input_command(
            Snowflake::default(),
            None,
            String::from("name"),
            String::from("descr"),
            None,
            None,
            None,
            None,
        );

        println!("{}", serde_json::to_string_pretty(&command).unwrap());
    }

    #[test]
    pub fn serialize_message_command() {
        let command = ApplicationCommand::new_message_command(
            Snowflake::default(),
            None,
            String::from("name"),
            None,
            None,
            None,
        );

        println!("{}", serde_json::to_string_pretty(&command).unwrap());
    }

    #[test]
    pub fn serialize_user_command() {
        let command = ApplicationCommand::new_user_command(
            Snowflake::default(),
            None,
            String::from("name"),
            None,
            None,
            None,
        );

        println!("{}", serde_json::to_string_pretty(&command).unwrap());
    }

    #[test]
    pub fn serialize_command_with_options() {
        let command = ApplicationCommand::ChatInputCommand(ChatInputCommand {
            details: CommandDetails {
                t: TypeField::<1>,
                id: None,
                application_id: Snowflake::default(),
                guild_id: None,
                name: String::from("name"),
                name_localizations: None,
                default_member_permissions: None,
                dm_permission: None,
                nsfw: None,
                version: None,
            },
            description: String::from("description"),
            description_localizations: None,
            options: Some(vec![ApplicationCommandOption::new_boolean_option(
                String::from("bool name"),
                String::from("bool desc"),
                false,
            )]),
        });

        println!("{}", serde_json::to_string_pretty(&command).unwrap());
    }

    #[test]
    pub fn deserialize_command() {
        let json = r#"{
            "id": "0",
            "type": 1,
            "application_id": "0",
            "name": "name",
            "description": "description",
            "options": [
              {
                "type": 5,
                "name": "bool name",
                "description": "bool desc",
                "required": false
              },
              {
                "type": 7,
                "name": "channel",
                "description": "desc",
                "required": false
              }
            ]
          }"#;

        println!(
            "{:#?}",
            serde_json::from_str::<ApplicationCommand>(json).unwrap()
        );
    }
}
