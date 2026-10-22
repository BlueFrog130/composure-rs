use serde_json::Value;

use crate::types::InteractionResponse;

/// Reading an [`InteractionResponse`] in assertions, through its JSON (what Discord receives).
/// Components are found anywhere in the message: in action rows, sections and containers.
pub trait ResponseExt {
    /// The response as Discord receives it
    fn json(&self) -> Value;

    /// The [response type](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-response-object-interaction-callback-type):
    /// 4 message, 5 deferred message, 6 deferred update, 7 update, 8 autocomplete, 9 modal
    fn response_type(&self) -> u8 {
        self.json()["type"].as_u64().unwrap_or_default() as u8
    }

    /// The message's `content`, else its text displays' content joined with newlines (Components
    /// V2); `None` without either
    fn text(&self) -> Option<String> {
        let json = self.json();
        if let Some(content) = json["data"]["content"].as_str() {
            return Some(content.to_string());
        }
        let texts: Vec<_> = components_of(&json)
            .into_iter()
            .filter(|component| component["type"] == 10)
            .filter_map(|component| component["content"].as_str().map(str::to_string))
            .collect();
        (!texts.is_empty()).then(|| texts.join("\n"))
    }

    /// Whether the message (or deferral) is only shown to the invoking user
    fn is_ephemeral(&self) -> bool {
        self.json()["data"]["flags"]
            .as_u64()
            .is_some_and(|flags| flags & 64 != 0)
    }

    /// Every component, depth first
    fn components(&self) -> Vec<Value> {
        components_of(&self.json())
    }

    /// The first component labeled `label` (a button, mostly)
    fn button(&self, label: &str) -> Option<Value> {
        self.components()
            .into_iter()
            .find(|component| component["label"] == label)
    }

    /// The `custom_id` of the component labeled `label`
    fn custom_id(&self, label: &str) -> Option<String> {
        self.button(label)?["custom_id"]
            .as_str()
            .map(str::to_string)
    }

    /// The message's embeds
    fn embeds(&self) -> Vec<Value> {
        array(&self.json()["data"]["embeds"])
    }

    /// An autocomplete response's choices
    fn choices(&self) -> Vec<Value> {
        array(&self.json()["data"]["choices"])
    }
}

impl ResponseExt for InteractionResponse {
    fn json(&self) -> Value {
        serde_json::to_value(self).expect("responses serialize")
    }
}

/// A response as JSON, like one read back from an Adapter over HTTP
impl ResponseExt for Value {
    fn json(&self) -> Value {
        self.clone()
    }
}

fn array(value: &Value) -> Vec<Value> {
    value.as_array().cloned().unwrap_or_default()
}

/// Every object with a `type` under `data.components`, depth first
fn components_of(json: &Value) -> Vec<Value> {
    fn walk(value: &Value, out: &mut Vec<Value>) {
        match value {
            Value::Object(map) => {
                if map.get("type").is_some_and(Value::is_u64) {
                    out.push(value.clone());
                }
                map.values().for_each(|value| walk(value, out));
            }
            Value::Array(items) => items.iter().for_each(|value| walk(value, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(&json["data"]["components"], &mut out);
    out
}
