use serde_json::{Map, Value, json};

use crate::types::{Interaction, Permissions, Snowflake, User};

use super::{APPLICATION_ID, CHANNEL_ID, INTERACTION_TOKEN, MESSAGE_ID, users};

/// An Interaction being built: invoked by [`users::alice`] in a DM in [`CHANNEL_ID`] unless set
/// otherwise. Finish with [`build`](Self::build) (or pass it where an `Into<Interaction>` goes, like
/// [`run`](super::run)), [`to_json`](Self::to_json), or [`body`](Self::body) for signing.
#[must_use]
pub struct InteractionBuilder<D> {
    envelope: Envelope,
    data: D,
}

/// A change to the built JSON, from [`InteractionBuilder::json`]
type Edit = Box<dyn FnOnce(&mut Value)>;

struct Envelope {
    kind: u8,
    user: User,
    guild: Option<Snowflake>,
    permissions: Permissions,
    locale: Option<String>,
    channel: Snowflake,
    message: Option<Value>,
    edits: Vec<Edit>,
}

/// A Command or autocomplete Interaction's data, built by [`command`], [`user_command`],
/// [`message_command`] and [`autocomplete`]
pub struct CommandData {
    name: String,
    command_type: u8,
    target: Option<Snowflake>,
    group: Option<String>,
    subcommand: Option<String>,
    options: Vec<Value>,
    resolved: Map<String, Value>,
}

/// A component Interaction's data, built by [`component`]
pub struct ComponentData {
    custom_id: String,
    component_type: u8,
    values: Vec<String>,
    resolved: Map<String, Value>,
}

/// A modal submit's data, built by [`modal`]
pub struct ModalData {
    custom_id: String,
    components: Vec<Value>,
}

/// An Option's value, from `&str`/`String` (string), integers, `f64` (number) or `bool`
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    /// A string Option (type 3)
    String(String),
    /// An integer Option (type 4)
    Integer(i64),
    /// A number Option (type 10)
    Number(f64),
    /// A boolean Option (type 5)
    Boolean(bool),
}

impl OptionValue {
    fn json(self) -> (u8, Value) {
        match self {
            OptionValue::String(value) => (3, value.into()),
            OptionValue::Integer(value) => (4, value.into()),
            OptionValue::Number(value) => (10, value.into()),
            OptionValue::Boolean(value) => (5, value.into()),
        }
    }
}

impl From<&str> for OptionValue {
    fn from(value: &str) -> Self {
        OptionValue::String(value.into())
    }
}

impl From<String> for OptionValue {
    fn from(value: String) -> Self {
        OptionValue::String(value)
    }
}

impl From<f64> for OptionValue {
    fn from(value: f64) -> Self {
        OptionValue::Number(value)
    }
}

impl From<bool> for OptionValue {
    fn from(value: bool) -> Self {
        OptionValue::Boolean(value)
    }
}

macro_rules! integer_options {
    ($($int:ty),*) => {$(
        impl From<$int> for OptionValue {
            fn from(value: $int) -> Self {
                OptionValue::Integer(value.into())
            }
        }
    )*};
}

integer_options!(i8, i16, i32, i64, u8, u16, u32);

fn new<D>(kind: u8, data: D) -> InteractionBuilder<D> {
    InteractionBuilder {
        envelope: Envelope {
            kind,
            user: users::alice(),
            guild: None,
            permissions: Permissions::all(),
            locale: None,
            channel: CHANNEL_ID,
            message: None,
            edits: Vec::new(),
        },
        data,
    }
}

fn command_data(name: &str, command_type: u8) -> CommandData {
    CommandData {
        name: name.into(),
        command_type,
        target: None,
        group: None,
        subcommand: None,
        options: Vec::new(),
        resolved: Map::new(),
    }
}

/// The slash Command `name`, without Options yet
///
/// ```
/// use composure::testing;
///
/// let ban = testing::command("mod")
///     .subcommand("ban")
///     .user_option("user", &testing::users::bob())
///     .option("reason", "spam")
///     .option("days", 7)
///     .build();
/// # let _ = ban;
/// ```
pub fn command(name: &str) -> InteractionBuilder<CommandData> {
    new(2, command_data(name, 1))
}

/// The user Command `name` (a context menu entry) on `target`
pub fn user_command(name: &str, target: &User) -> InteractionBuilder<CommandData> {
    let mut data = command_data(name, 2);
    data.target = Some(target.id);
    data.resolved.insert(
        "users".into(),
        json!({ target.id.to_string(): user_json(target) }),
    );
    new(2, data)
}

/// The message Command `name` (a context menu entry) on a message ([`MESSAGE_ID`], by
/// [`users::alice`]) reading `content`
pub fn message_command(name: &str, content: &str) -> InteractionBuilder<CommandData> {
    let mut data = command_data(name, 3);
    data.target = Some(MESSAGE_ID);
    let message = json!({
        "id": MESSAGE_ID, "channel_id": CHANNEL_ID, "author": user_json(&users::alice()),
        "content": content, "timestamp": "2026-01-01T00:00:00.000000+00:00",
        "edited_timestamp": null, "tts": false, "mention_everyone": false, "mentions": [],
        "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0,
    });
    data.resolved.insert(
        "messages".into(),
        json!({ MESSAGE_ID.to_string(): message }),
    );
    new(2, data)
}

/// Autocomplete for the slash Command `name`; say which Option is being typed with
/// [`focused`](InteractionBuilder::focused)
pub fn autocomplete(name: &str) -> InteractionBuilder<CommandData> {
    new(4, command_data(name, 1))
}

/// A click on the button `custom_id`; a select with [`values`](InteractionBuilder::values)
pub fn component(custom_id: &str) -> InteractionBuilder<ComponentData> {
    new(
        3,
        ComponentData {
            custom_id: custom_id.into(),
            component_type: 2,
            values: Vec::new(),
            resolved: Map::new(),
        },
    )
}

/// Submitting the modal `custom_id`, without fields yet
///
/// ```
/// use composure::testing;
/// use serde_json::json;
///
/// let submit = testing::modal("feedback")
///     .field("topic", "Docs")
///     .select("kind", ["Idea"])
///     .input(json!({ "type": 23, "custom_id": "contact", "value": true }))
///     .build();
/// # let _ = submit;
/// ```
pub fn modal(custom_id: &str) -> InteractionBuilder<ModalData> {
    new(
        5,
        ModalData {
            custom_id: custom_id.into(),
            components: Vec::new(),
        },
    )
}

impl<D> InteractionBuilder<D> {
    /// Invoked by `user`
    pub fn by(mut self, user: &User) -> Self {
        self.envelope.user = user.clone();
        self
    }

    /// Invoked in the guild `guild_id` (by a member) instead of a DM
    pub fn in_guild(mut self, guild_id: Snowflake) -> Self {
        self.envelope.guild = Some(guild_id);
        self
    }

    /// The invoking member's permissions in the channel; every permission unless set. Only sent in
    /// guilds ([`in_guild`](Self::in_guild)).
    pub fn permissions(mut self, permissions: Permissions) -> Self {
        self.envelope.permissions = permissions;
        self
    }

    /// The invoking user's language, like `en-US`
    pub fn locale(mut self, locale: &str) -> Self {
        self.envelope.locale = Some(locale.into());
        self
    }

    /// Invoked in `channel_id` instead of [`CHANNEL_ID`]
    pub fn channel(mut self, channel_id: Snowflake) -> Self {
        self.envelope.channel = channel_id;
        self
    }

    /// The message the Interaction comes from: the one holding a clicked component
    pub fn message(mut self, message: Value) -> Self {
        self.envelope.message = Some(message);
        self
    }

    /// Changes the JSON once built, for anything the builder doesn't cover
    pub fn json(mut self, edit: impl FnOnce(&mut Value) + 'static) -> Self {
        self.envelope.edits.push(Box::new(edit));
        self
    }
}

impl InteractionBuilder<CommandData> {
    /// An Option, of the type its value implies (string, integer, number or boolean)
    pub fn option(mut self, name: &str, value: impl Into<OptionValue>) -> Self {
        let (option_type, value) = value.into().json();
        self.data
            .options
            .push(json!({ "name": name, "type": option_type, "value": value }));
        self
    }

    /// A user Option, resolved to `user`
    pub fn user_option(mut self, name: &str, user: &User) -> Self {
        self.data
            .options
            .push(json!({ "name": name, "type": 6, "value": user.id }));
        self.resolved(json!({ "users": { user.id.to_string(): user_json(user) } }))
    }

    /// An Option as raw JSON (`{ "name", "type", "value" }`), for channels, roles, attachments...
    /// with their [`resolved`](Self::resolved) data
    pub fn option_json(mut self, option: Value) -> Self {
        self.data.options.push(option);
        self
    }

    /// The string Option `name` the user is typing `typed` into, for [`autocomplete`]
    pub fn focused(mut self, name: &str, typed: &str) -> Self {
        self.data
            .options
            .push(json!({ "name": name, "type": 3, "value": typed, "focused": true }));
        self
    }

    /// Invokes the subcommand `name`; the Options added go to it
    pub fn subcommand(mut self, name: &str) -> Self {
        self.data.subcommand = Some(name.into());
        self
    }

    /// Invokes a subcommand in the group `name`; set the subcommand with
    /// [`subcommand`](Self::subcommand)
    pub fn group(mut self, name: &str) -> Self {
        self.data.group = Some(name.into());
        self
    }

    /// Merges `resolved` (`{ "users": {..}, "channels": {..}, .. }`) into the resolved data
    pub fn resolved(mut self, resolved: Value) -> Self {
        merge(&mut self.data.resolved, resolved);
        self
    }
}

impl InteractionBuilder<ComponentData> {
    /// The values picked in a select (a string select unless
    /// [`component_type`](Self::component_type) says otherwise)
    pub fn values<I: IntoIterator<Item = S>, S: Into<String>>(mut self, values: I) -> Self {
        if self.data.component_type == 2 {
            self.data.component_type = 3;
        }
        self.data.values = values.into_iter().map(Into::into).collect();
        self
    }

    /// The [component type](https://discord.com/developers/docs/components/reference#component-object-component-types):
    /// 2 button, 3 string select, 5 user select, ...
    pub fn component_type(mut self, component_type: u8) -> Self {
        self.data.component_type = component_type;
        self
    }

    /// Merges `resolved` (`{ "users": {..}, .. }`, for user and other selects) into the resolved
    /// data
    pub fn resolved(mut self, resolved: Value) -> Self {
        merge(&mut self.data.resolved, resolved);
        self
    }
}

impl InteractionBuilder<ModalData> {
    /// A text input `custom_id` holding `value`
    pub fn field(self, custom_id: &str, value: &str) -> Self {
        self.input(json!({ "type": 4, "custom_id": custom_id, "value": value }))
    }

    /// A string select (or radio group) `custom_id` with `values` picked
    pub fn select<I: IntoIterator<Item = S>, S: Into<String>>(
        self,
        custom_id: &str,
        values: I,
    ) -> Self {
        let values: Vec<String> = values.into_iter().map(Into::into).collect();
        self.input(json!({ "type": 3, "custom_id": custom_id, "values": values }))
    }

    /// Any submitted input as raw JSON (`{ "type", "custom_id", .. }`), in a Label like every
    /// modal field
    pub fn input(mut self, component: Value) -> Self {
        let id = self.data.components.len() + 1;
        self.data
            .components
            .push(json!({ "type": 18, "id": id, "component": component }));
        self
    }
}

impl CommandData {
    fn json(self) -> Value {
        let options = match (self.group, self.subcommand) {
            (Some(group), Some(subcommand)) => vec![json!({
                "name": group, "type": 2,
                "options": [{ "name": subcommand, "type": 1, "options": self.options }],
            })],
            (None, Some(subcommand)) => {
                vec![json!({ "name": subcommand, "type": 1, "options": self.options })]
            }
            _ => self.options,
        };
        let mut data =
            json!({ "id": "1052358444704862218", "name": self.name, "type": self.command_type });
        if !options.is_empty() {
            data["options"] = options.into();
        }
        if let Some(target) = self.target {
            data["target_id"] = json!(target);
        }
        if !self.resolved.is_empty() {
            data["resolved"] = self.resolved.into();
        }
        data
    }
}

impl ComponentData {
    fn json(self) -> Value {
        let mut data =
            json!({ "custom_id": self.custom_id, "component_type": self.component_type });
        if self.component_type != 2 {
            data["values"] = self.values.into();
        }
        if !self.resolved.is_empty() {
            data["resolved"] = self.resolved.into();
        }
        data
    }
}

impl ModalData {
    fn json(self) -> Value {
        json!({ "custom_id": self.custom_id, "components": self.components })
    }
}

macro_rules! finish {
    ($($data:ty),*) => {$(
        impl InteractionBuilder<$data> {
            /// The Interaction as Discord sends it
            pub fn to_json(self) -> Value {
                self.envelope.finish(self.data.json())
            }

            /// The request body Discord would send, to sign with
            /// [`TestKey::signed`](super::TestKey::signed)
            pub fn body(self) -> Vec<u8> {
                serde_json::to_vec(&self.to_json()).expect("JSON values serialize")
            }

            /// The Interaction, parsed as an Adapter parses it
            pub fn build(self) -> Interaction {
                serde_json::from_slice(&self.body()).expect("built Interactions parse")
            }
        }

        impl From<InteractionBuilder<$data>> for Interaction {
            fn from(builder: InteractionBuilder<$data>) -> Self {
                builder.build()
            }
        }
    )*};
}

finish!(CommandData, ComponentData, ModalData);

impl Envelope {
    fn finish(self, data: Value) -> Value {
        let mut json = json!({
            "id": "1100173248714518568", "application_id": APPLICATION_ID, "type": self.kind,
            "token": INTERACTION_TOKEN, "version": 1, "channel_id": self.channel, "data": data,
            "entitlements": [],
        });
        let user = user_json(&self.user);
        match self.guild {
            Some(guild) => {
                json["guild_id"] = json!(guild);
                json["member"] = json!({
                    "user": user, "nick": null, "avatar": null, "roles": [],
                    "joined_at": "2021-01-12T21:18:10.481000+00:00", "premium_since": null,
                    "deaf": false, "mute": false, "flags": 0, "pending": false,
                    "permissions": self.permissions, "communication_disabled_until": null,
                });
            }
            None => json["user"] = user,
        }
        if let Some(locale) = self.locale {
            json["locale"] = locale.into();
        }
        if let Some(message) = self.message {
            json["message"] = message;
        }
        for edit in self.edits {
            edit(&mut json);
        }
        json
    }
}

/// `user` as Discord sends it
fn user_json(user: &User) -> Value {
    json!({
        "id": user.id, "username": user.username, "discriminator": user.discriminator,
        "global_name": user.global_name, "avatar": user.avatar, "public_flags": user.public_flags,
    })
}

/// Adds `value`'s maps to `resolved`'s, key by key
fn merge(resolved: &mut Map<String, Value>, value: Value) {
    let Value::Object(value) = value else {
        panic!("resolved data is an object, not {value}")
    };
    for (key, entries) in value {
        match (resolved.get_mut(&key), entries) {
            (Some(Value::Object(existing)), Value::Object(entries)) => existing.extend(entries),
            (_, entries) => {
                resolved.insert(key, entries);
            }
        }
    }
}
