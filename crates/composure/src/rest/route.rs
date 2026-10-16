use std::fmt::{self, Display, Write};

use super::Method;

/// One Discord Endpoint: method + path template exactly as in Discord's OpenAPI spec, so the spec
/// check can match it (parameter names may differ, positions may not).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    pub method: Method,
    pub path: &'static str,
}

impl Route {
    /// `base` + `path` with each `{param}` replaced, in order, by `params`
    pub(crate) fn url(&self, base: &str, params: &[&dyn Display]) -> String {
        let mut url = String::with_capacity(base.len() + self.path.len() + 32);
        url.push_str(base);
        let mut params = params.iter();
        let mut rest = self.path;
        while let Some(start) = rest.find('{') {
            url.push_str(&rest[..start]);
            let end = start + rest[start..].find('}').expect("unclosed route parameter");
            let param = params.next().expect("missing route parameter");
            write!(url, "{param}").expect("writing to a String");
            rest = &rest[end + 1..];
        }
        url.push_str(rest);
        debug_assert!(params.next().is_none(), "extra route parameter");
        url
    }
}

impl Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.method, self.path)
    }
}

/// Declares Endpoint [`Route`] constants and, for the spec check, a `ROUTES` table listing them.
macro_rules! routes {
    ($($name:ident = $method:ident $path:literal;)*) => {
        $(pub(crate) const $name: $crate::rest::Route = $crate::rest::Route {
            method: $crate::rest::Method::$method,
            path: $path,
        };)*

        #[cfg(test)]
        pub(crate) const ROUTES: &[$crate::rest::Route] = &[$($name),*];
    };
}

pub(crate) use routes;

/// A path parameter that is free text (emoji, names): percent-encoded, keeping `:` for
/// custom emoji (`name:id`)
pub(crate) struct PathSegment<'a>(pub(crate) &'a str);

impl Display for PathSegment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b':' => {
                    f.write_char(byte as char)?
                }
                _ => write!(f, "%{byte:02X}")?,
            }
        }
        Ok(())
    }
}

/// Appends `?key=value&...` to a URL, percent-encoding values
pub(crate) struct Query<'a> {
    url: &'a mut String,
    first: bool,
}

#[allow(dead_code)] // only some Endpoint features use queries
impl<'a> Query<'a> {
    pub(crate) fn new(url: &'a mut String) -> Self {
        let first = !url.contains('?');
        Self { url, first }
    }

    pub(crate) fn push(&mut self, key: &str, value: impl Display) -> &mut Self {
        self.url.push(if self.first { '?' } else { '&' });
        self.first = false;
        self.url.push_str(key);
        self.url.push('=');
        let value = value.to_string();
        for byte in value.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    self.url.push(byte as char)
                }
                _ => write!(self.url, "%{byte:02X}").expect("writing to a String"),
            }
        }
        self
    }

    pub(crate) fn push_opt(&mut self, key: &str, value: Option<impl Display>) -> &mut Self {
        if let Some(value) = value {
            self.push(key, value);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_parameters_in_order() {
        let route = Route {
            method: Method::Get,
            path: "/webhooks/{application_id}/{interaction_token}/messages/@original",
        };
        assert_eq!(
            route.url("https://x", &[&1, &"tok"]),
            "https://x/webhooks/1/tok/messages/@original"
        );
    }

    #[test]
    fn query_encodes_values() {
        let mut url = String::from("https://x/a");
        Query::new(&mut url)
            .push("wait", true)
            .push_opt("thread_id", None::<u64>)
            .push("q", "a b&c");
        assert_eq!(url, "https://x/a?wait=true&q=a%20b%26c");
    }
}
