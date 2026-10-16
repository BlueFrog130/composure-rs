//! Parsing and validation shared by `#[command]` and `#[group]`.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{Attribute, Expr, ExprLit, Ident, Lit, LitStr, Token, meta::ParseNestedMeta};

/// Settings accepted by both `#[command(...)]` and `#[group(...)]`
#[derive(Default)]
pub struct Settings {
    pub name: Option<LitStr>,
    pub desc: Option<LitStr>,
    pub scope: Option<LitStr>,
    pub permissions: Option<Expr>,
    pub contexts: Option<Vec<Ident>>,
    pub integration_types: Option<Vec<Ident>>,
    pub nsfw: Option<bool>,
}

impl Settings {
    /// Parses one shared setting; `false` if `meta` is not one of them
    pub fn parse(&mut self, meta: &ParseNestedMeta) -> syn::Result<bool> {
        if meta.path.is_ident("name") {
            self.name = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("desc") {
            self.desc = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("scope") {
            let scope: LitStr = meta.value()?.parse()?;
            validate_scope(&scope)?;
            self.scope = Some(scope);
        } else if meta.path.is_ident("permissions") {
            self.permissions = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("contexts") {
            self.contexts = Some(ident_list(meta)?);
        } else if meta.path.is_ident("integration_types") {
            self.integration_types = Some(ident_list(meta)?);
        } else if meta.path.is_ident("nsfw") {
            self.nsfw = Some(if meta.input.peek(Token![=]) {
                meta.value()?.parse::<syn::LitBool>()?.value
            } else {
                true
            });
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    /// `::composure::command::CommandMeta` for these settings
    pub fn meta_tokens(&self, name: &str, description: &str) -> TokenStream {
        let mut fields = Vec::new();
        if let Some(permissions) = &self.permissions {
            fields.push(quote! {
                default_member_permissions: ::core::option::Option::Some({
                    #[allow(unused_imports)]
                    use ::composure::models::Permissions;
                    #permissions
                }),
            });
        }
        if let Some(types) = &self.integration_types {
            fields.push(quote! {
                integration_types: ::core::option::Option::Some(
                    &[#(::composure::models::ApplicationIntegrationType::#types),*]
                ),
            });
        }
        if let Some(contexts) = &self.contexts {
            fields.push(quote! {
                contexts: ::core::option::Option::Some(
                    &[#(::composure::models::InteractionContextType::#contexts),*]
                ),
            });
        }
        if let Some(nsfw) = self.nsfw {
            fields.push(quote! { nsfw: ::core::option::Option::Some(#nsfw), });
        }
        quote! {
            ::composure::command::CommandMeta {
                #(#fields)*
                ..::composure::command::CommandMeta::new(#name, #description)
            }
        }
    }

    pub fn scope_tokens(&self) -> TokenStream {
        match &self.scope {
            Some(scope) => quote! { ::core::option::Option::Some(#scope) },
            None => quote! { ::core::option::Option::None },
        }
    }
}

/// `(A, B, C)` after a setting name
pub fn ident_list(meta: &ParseNestedMeta) -> syn::Result<Vec<Ident>> {
    let mut idents = Vec::new();
    meta.parse_nested_meta(|item| {
        idents.push(
            item.path
                .get_ident()
                .cloned()
                .ok_or_else(|| item.error("expected a variant name"))?,
        );
        Ok(())
    })?;
    Ok(idents)
}

/// Doc comment lines joined into one description
pub fn doc_description(attrs: &[Attribute]) -> Option<String> {
    let lines: Vec<String> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| match &attr.meta {
            syn::Meta::NameValue(nv) => match &nv.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) => Some(s.value().trim().to_string()),
                _ => None,
            },
            _ => None,
        })
        .filter(|line| !line.is_empty())
        .collect();
    (!lines.is_empty()).then(|| lines.join(" "))
}

/// The description from `desc = "..."` or the doc comment; required and 1-100 characters
pub fn description(settings: &Settings, attrs: &[Attribute], span: Span) -> syn::Result<String> {
    let (description, span) = match &settings.desc {
        Some(desc) => (desc.value(), desc.span()),
        None => (
            doc_description(attrs).ok_or_else(|| {
                syn::Error::new(
                    span,
                    "missing description: add a doc comment or `desc = \"...\"`",
                )
            })?,
            span,
        ),
    };
    validate_description(&description, span)?;
    Ok(description)
}

pub fn validate_description(description: &str, span: Span) -> syn::Result<()> {
    let len = description.chars().count();
    if !(1..=100).contains(&len) {
        return Err(syn::Error::new(
            span,
            format!("description must be 1-100 characters, this one is {len}"),
        ));
    }
    Ok(())
}

/// [Chat-input naming](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-naming):
/// 1-32 lowercase letters, digits, `-`, or `_`
pub fn validate_chat_name(name: &str, span: Span) -> syn::Result<()> {
    let len = name.chars().count();
    let valid_chars = name
        .chars()
        .all(|c| (c.is_alphanumeric() && !c.is_uppercase()) || c == '-' || c == '_' || c == '\'');
    if !(1..=32).contains(&len) || !valid_chars {
        return Err(syn::Error::new(
            span,
            format!(
                "`{name}` is not a valid command or option name: use 1-32 lowercase letters, digits, `-` or `_`"
            ),
        ));
    }
    Ok(())
}

/// User and message command names: 1-32 characters, spaces and capitals allowed
pub fn validate_context_name(name: &str, span: Span) -> syn::Result<()> {
    let len = name.chars().count();
    if !(1..=32).contains(&len) {
        return Err(syn::Error::new(
            span,
            format!("`{name}` is not a valid command name: use 1-32 characters"),
        ));
    }
    Ok(())
}

/// Scopes become `COMPOSURE_SCOPE_<NAME>` env vars, so keep them to letters, digits, `-`, `_`
fn validate_scope(scope: &LitStr) -> syn::Result<()> {
    let value = scope.value();
    if value.is_empty()
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(syn::Error::new(
            scope.span(),
            "scope must be non-empty ASCII letters, digits, `-` or `_`",
        ));
    }
    Ok(())
}

/// `RoleTools` -> `role_tools`, `HTTPTools` -> `http_tools`
pub fn snake_case(ident: &Ident) -> String {
    let chars: Vec<char> = ident.to_string().chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() && i > 0 {
            let previous = chars[i - 1];
            let next_is_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            // A new word starts after a lowercase letter or digit, or at the last capital of
            // an acronym followed by lowercase
            if previous.is_lowercase()
                || previous.is_ascii_digit()
                || (previous.is_uppercase() && next_is_lower)
            {
                out.push('_');
            }
        }
        out.extend(c.to_lowercase());
    }
    out
}

/// An integer literal, optionally negated
pub fn integer(expr: &Expr) -> syn::Result<i64> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        }) => i.base10_parse(),
        Expr::Unary(syn::ExprUnary {
            op: syn::UnOp::Neg(_),
            expr,
            ..
        }) => integer(expr).map(|v| -v),
        _ => Err(syn::Error::new_spanned(
            expr.to_token_stream(),
            "expected an integer literal",
        )),
    }
}

/// A numeric literal, optionally negated, as `f64`
pub fn number(expr: &Expr) -> syn::Result<f64> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        }) => i.base10_parse::<i64>().map(|v| v as f64),
        Expr::Lit(ExprLit {
            lit: Lit::Float(f), ..
        }) => f.base10_parse::<f64>(),
        Expr::Unary(syn::ExprUnary {
            op: syn::UnOp::Neg(_),
            expr,
            ..
        }) => number(expr).map(|v| -v),
        _ => Err(syn::Error::new_spanned(
            expr.to_token_stream(),
            "expected a number literal",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_keeps_acronyms_together() {
        let case = |name| snake_case(&Ident::new(name, Span::call_site()));
        assert_eq!(case("Mod"), "mod");
        assert_eq!(case("RoleTools"), "role_tools");
        assert_eq!(case("HTTPTools"), "http_tools");
        assert_eq!(case("ToolsV2"), "tools_v2");
        assert_eq!(case("IOError"), "io_error");
    }
}
