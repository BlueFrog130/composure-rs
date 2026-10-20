mod builder;

pub use builder::*;
pub use composure::definition::*;

#[cfg(test)]
mod tests {
    use composure::types::{ApplicationIntegrationType, InteractionContextType, TypeField};
    use serde_json::json;

    use super::*;

    #[test]
    pub fn serialize_command() {
        let command = ApplicationCommand::new_chat_input_command(
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
        let command =
            ApplicationCommand::new_message_command(String::from("name"), None, None, None);

        println!("{}", serde_json::to_string_pretty(&command).unwrap());
    }

    #[test]
    pub fn serialize_user_command() {
        let command = ApplicationCommand::new_user_command(String::from("name"), None, None, None);

        println!("{}", serde_json::to_string_pretty(&command).unwrap());
    }

    #[test]
    pub fn serialize_command_with_options() {
        let command = ApplicationCommand::ChatInputCommand(ChatInputCommand {
            details: CommandDetails {
                t: TypeField::<1>,
                id: None,
                application_id: None,
                guild_id: None,
                name: String::from("name"),
                name_localizations: None,
                default_member_permissions: None,
                dm_permission: None,
                integration_types: None,
                contexts: None,
                nsfw: None,
                version: None,
            },
            description: String::from("description"),
            description_localizations: None,
            options: Some(vec![ApplicationCommandOption::new_boolean_option(
                String::from("bool name"),
                String::from("bool desc"),
                None,
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

    #[test]
    pub fn serialize_contexts_and_integration_types() {
        let command = CommandBuilder::new()
            .name("hello")
            .description("Say hello")
            .with_integration_types(&[
                ApplicationIntegrationType::GuildInstall,
                ApplicationIntegrationType::UserInstall,
            ])
            .with_contexts(&[
                InteractionContextType::Guild,
                InteractionContextType::BotDm,
                InteractionContextType::PrivateChannel,
            ])
            .build_chat_command();

        assert_eq!(
            serde_json::to_value(&command).unwrap(),
            json!({
                "type": 1,
                "name": "hello",
                "description": "Say hello",
                "integration_types": [0, 1],
                "contexts": [0, 1, 2],
            })
        );
    }

    #[test]
    pub fn serialize_omits_unset_contexts() {
        let command = CommandBuilder::new()
            .name("hello")
            .description("Say hello")
            .build_chat_command();

        let value = serde_json::to_value(&command).unwrap();
        assert!(value.get("integration_types").is_none());
        assert!(value.get("contexts").is_none());
    }

    #[test]
    pub fn deserialize_contexts_and_integration_types() {
        let command: ApplicationCommand = serde_json::from_value(json!({
            "id": "1052358444704862218",
            "application_id": "1052322265397739523",
            "version": "1100155827400229026",
            "default_member_permissions": null,
            "type": 1,
            "name": "hello",
            "description": "Say hello",
            "dm_permission": true,
            "contexts": [0, 2],
            "integration_types": [1],
            "nsfw": false
        }))
        .unwrap();

        let details = &command.as_chat_input_command().unwrap().details;
        assert_eq!(
            details.contexts,
            Some(vec![
                InteractionContextType::Guild,
                InteractionContextType::PrivateChannel
            ])
        );
        assert_eq!(
            details.integration_types,
            Some(vec![ApplicationIntegrationType::UserInstall])
        );
    }

    #[test]
    pub fn deserialize_null_contexts() {
        let command: ApplicationCommand = serde_json::from_value(json!({
            "id": "1052358444704862218",
            "application_id": "1052322265397739523",
            "version": "1100155827400229026",
            "type": 2,
            "name": "inspect",
            "contexts": null
        }))
        .unwrap();

        let details = command.as_user_command().unwrap();
        assert_eq!(details.contexts, None);
        assert_eq!(details.integration_types, None);
    }
}
