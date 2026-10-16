//! Test-only handler that answers every interaction with a plain-text description of what it
//! parsed (option values, resolved users/channels/roles/attachments, component values, modal
//! inputs), so the fixtures can assert on exactly what the types produced.

use composure::types::{
    ApplicationCommandInteraction, ApplicationCommandInteractionData,
    ApplicationCommandInteractionDataOption, ApplicationCommandOptionChoice,
    ApplicationCommandOptionChoiceValue, ApplicationCommandType, InteractionResponse,
    MessageComponentInteraction, ModalSubmitInteraction, OptionList,
};
use composure_adapter_cloudflare::{HandlerError, InteractionHandler};

/// Describes every interaction it receives
pub struct RegressionBot;

impl<S> InteractionHandler<S> for RegressionBot {
    async fn command(
        &self,
        command: ApplicationCommandInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        let data = &command.data;
        let content = match data.t {
            ApplicationCommandType::ChatInput if data.name == "regress-fail" => {
                return Err(HandlerError::new("regress-fail always fails"));
            }
            ApplicationCommandType::ChatInput if data.name == "regress-contexts" => {
                let user = command
                    .common
                    .invoking_user()
                    .map_or("nobody", |u| u.display_name());
                format!(
                    "regress-contexts by {user} in {:?} owners={}",
                    command.common.context,
                    command.common.authorizing_integration_owners.len()
                )
            }
            ApplicationCommandType::ChatInput => {
                let mut parts = vec![data.name.clone()];
                if let Some(options) = &data.options {
                    describe_options(data, options, &mut parts);
                }
                parts.join(" ")
            }
            ApplicationCommandType::User => {
                let target = data
                    .target_id
                    .as_ref()
                    .and_then(|id| data.resolved_user(id))
                    .map_or("unresolved", |u| u.display_name());
                format!("{} -> {target}", data.name)
            }
            ApplicationCommandType::Message => {
                let target = data
                    .target_id
                    .as_ref()
                    .and_then(|id| data.resolved_message(id))
                    .map_or("unresolved", |m| m.content.as_str());
                format!("{} -> {target}", data.name)
            }
        };

        Ok(InteractionResponse::respond_with_message(content))
    }

    async fn component(
        &self,
        component: MessageComponentInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        let data = &component.data;
        let mut content = format!("component {}", data.custom_id);
        if let Some(values) = &data.values {
            content.push(' ');
            content.push_str(&values.join(","));
        }
        if let Some(users) = data.resolved.as_ref().and_then(|r| r.users.as_ref()) {
            let mut names: Vec<&str> = users.values().map(|u| u.display_name()).collect();
            names.sort();
            content.push_str(&format!(" users={}", names.join(",")));
        }
        Ok(InteractionResponse::respond_with_message(content))
    }

    async fn autocomplete(
        &self,
        autocomplete: ApplicationCommandInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        let Some(focused) = autocomplete.data.focused_option() else {
            return Ok(InteractionResponse::respond_with_autocomplete_choices(
                Vec::new(),
            ));
        };

        let value = match focused.t {
            4 => focused
                .value
                .parse()
                .ok()
                .map(ApplicationCommandOptionChoiceValue::Integer),
            10 => focused
                .value
                .parse()
                .ok()
                .map(ApplicationCommandOptionChoiceValue::Double),
            _ => Some(ApplicationCommandOptionChoiceValue::String(
                focused.value.clone(),
            )),
        };

        let choices = value
            .map(|value| ApplicationCommandOptionChoice {
                name: format!("{}: {}", focused.name, focused.value),
                name_localizations: None,
                value,
            })
            .into_iter()
            .collect();

        Ok(InteractionResponse::respond_with_autocomplete_choices(
            choices,
        ))
    }

    async fn modal_submit(
        &self,
        modal: ModalSubmitInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        let data = &modal.data;
        let title = data.value("title").unwrap_or("-");
        let body = data.value("body").unwrap_or("-");
        let mut content = format!("modal {} title={title} body={body}", data.custom_id);
        if let Some(values) = data.values("mood") {
            content.push_str(&format!(" mood={}", values.join(",")));
        }
        Ok(InteractionResponse::respond_with_message(content))
    }
}

/// `name=value` for each option, recursing into subcommands and groups by name.
fn describe_options(
    data: &ApplicationCommandInteractionData,
    options: &OptionList,
    parts: &mut Vec<String>,
) {
    for option in options.iter() {
        use ApplicationCommandInteractionDataOption as O;
        let described = match option {
            O::SubcommandGroup(group) => {
                parts.push(group.name.clone());
                parts.push(group.subcommand.name.clone());
                describe_options(data, &group.subcommand.options, parts);
                continue;
            }
            O::Subcommand(sub) => {
                parts.push(sub.name.clone());
                describe_options(data, &sub.options, parts);
                continue;
            }
            O::String(o) => o.value.clone(),
            O::Integer(o) => o.value.to_string(),
            O::Number(o) => o.value.to_string(),
            O::Boolean(o) => o.value.to_string(),
            O::User(o) => data
                .resolved_user(&o.value)
                .map_or("?".into(), |u| format!("@{}", u.display_name())),
            O::Channel(o) => data
                .resolved_channel(&o.value)
                .and_then(|c| c.name.clone())
                .map_or("?".into(), |name| format!("#{name}")),
            O::Role(o) => data
                .resolved_role(&o.value)
                .map_or("?".into(), |r| format!("@&{}", r.name)),
            O::Mentionable(o) => match (data.resolved_user(&o.value), data.resolved_role(&o.value))
            {
                (Some(user), _) => format!("@{}", user.display_name()),
                (None, Some(role)) => format!("@&{}", role.name),
                (None, None) => "?".into(),
            },
            O::Attachment(o) => data
                .resolved_attachment(&o.value)
                .map_or("?".into(), |a| a.filename.clone()),
            O::Focused(o) => format!("focused:{}", o.value),
        };
        parts.push(format!("{}={described}", option.name()));
    }
}
