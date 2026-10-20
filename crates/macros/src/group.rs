//! `#[group(subcommands(...))]`: makes a unit struct a Command that routes to its subcommands.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Fields, ItemStruct, Path};

use crate::meta::{self, Settings};

pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut settings = Settings::default();
    let mut subcommands: Vec<Path> = Vec::new();
    let parser = syn::meta::parser(|meta| {
        if settings.parse(&meta)? {
            Ok(())
        } else if meta.path.is_ident("subcommands") {
            meta.parse_nested_meta(|item| {
                subcommands.push(item.path.clone());
                Ok(())
            })
        } else {
            Err(meta.error(
                "unknown setting; expected one of subcommands, name, desc, scope, permissions, contexts, integration_types, nsfw",
            ))
        }
    });
    syn::parse::Parser::parse2(parser, args)?;

    let item: ItemStruct = syn::parse2(item)?;
    let ident = &item.ident;
    let span = ident.span();
    if !matches!(item.fields, Fields::Unit) || !item.generics.params.is_empty() {
        return Err(syn::Error::new(
            span,
            "groups must be unit structs: `struct Admin;`",
        ));
    }
    if subcommands.is_empty() {
        return Err(syn::Error::new(
            span,
            "groups need subcommands: `#[group(subcommands(ban, kick))]`",
        ));
    }
    if subcommands.len() > 25 {
        return Err(syn::Error::new(
            span,
            "groups can have at most 25 subcommands",
        ));
    }

    let name = match &settings.name {
        Some(name) => {
            meta::validate_chat_name(&name.value(), name.span())?;
            name.value()
        }
        None => {
            let name = meta::snake_case(ident);
            meta::validate_chat_name(&name, span)?;
            name
        }
    };
    let description = meta::description(&settings, &item.attrs, span)?;
    let settings_meta = settings.meta_tokens(&name, &description);
    let scope = settings.scope_tokens();

    let checks = subcommands.iter().map(|sub| {
        quote_spanned! {sub.span()=>
            ::core::assert!(
                <#sub as ::composure::command::SubcommandDefinition>::DEPTH <= 1,
                "groups nest at most one level: command > group > subcommand",
            );
            ::core::assert!(
                <#sub as ::composure::command::CommandDefinition>::SCOPE.is_none(),
                "`scope` is only allowed on top-level commands and groups",
            );
        }
    });
    let depths = subcommands.iter().map(|sub| {
        quote! {
            if <#sub as ::composure::command::SubcommandDefinition>::DEPTH > depth {
                depth = <#sub as ::composure::command::SubcommandDefinition>::DEPTH;
            }
        }
    });
    let routes = subcommands.iter().map(|sub| {
        quote! {
            if name == <#sub as ::composure::command::CommandDefinition>::NAME {
                return <#sub as ::composure::command::Command<__S>>::invoke(invocation).await;
            }
        }
    });
    let autocomplete_routes = subcommands.iter().map(|sub| {
        quote! {
            if name == <#sub as ::composure::command::CommandDefinition>::NAME {
                return <#sub as ::composure::command::Command<__S>>::autocomplete(invocation).await;
            }
        }
    });

    let route = crate::app::command_route(ident);

    Ok(quote! {
        #item

        // DEPTH is often 0, which clippy flags as an absurd comparison once the check is
        // spanned to user code
        #[allow(clippy::absurd_extreme_comparisons)]
        const _: () = {
            #(#checks)*
            ::composure::command::check_commands(&[#((
                <#subcommands as ::composure::command::CommandDefinition>::NAME,
                <#subcommands as ::composure::command::CommandDefinition>::KIND,
                <#subcommands as ::composure::command::CommandDefinition>::SCOPE,
            )),*]);
        };

        impl #ident {
            #[doc(hidden)]
            fn __meta() -> ::composure::command::CommandMeta {
                #settings_meta
            }

            #[doc(hidden)]
            fn __subcommands() -> ::std::vec::Vec<::composure::definition::ApplicationCommandOption> {
                ::std::vec![#(
                    <#subcommands as ::composure::command::SubcommandDefinition>::option_definition()
                ),*]
            }
        }

        impl ::composure::command::CommandDefinition for #ident {
            const NAME: &'static str = #name;
            const KIND: ::composure::types::ApplicationCommandType =
                ::composure::types::ApplicationCommandType::ChatInput;
            const SCOPE: ::core::option::Option<&'static str> = #scope;

            fn definition() -> ::composure::definition::ApplicationCommand {
                Self::__meta().chat_input(Self::__subcommands())
            }
        }

        impl ::composure::command::SubcommandDefinition for #ident {
            const DEPTH: u8 = {
                let mut depth = 0;
                #(#depths)*
                depth + 1
            };

            fn option_definition() -> ::composure::definition::ApplicationCommandOption {
                Self::__meta().subcommand_group(Self::__subcommands())
            }
        }

        impl<__S: ::core::marker::Sync> ::composure::command::Command<__S> for #ident
        where
            #(#subcommands: ::composure::command::Command<__S>,)*
        {
            async fn invoke(
                invocation: ::composure::command::Invocation<'_, __S>,
            ) -> ::core::result::Result<
                ::composure::types::InteractionResponse,
                ::composure::command::HandlerError,
            > {
                let ::core::option::Option::Some((name, invocation)) = invocation.route() else {
                    return ::core::result::Result::Err(
                        ::composure::command::HandlerError::unknown_command(
                            <Self as ::composure::command::CommandDefinition>::NAME,
                        ),
                    );
                };
                #(#routes)*
                ::core::result::Result::Err(::composure::command::HandlerError::unknown_command(name))
            }

            async fn autocomplete(
                invocation: ::composure::command::Invocation<'_, __S>,
            ) -> ::core::option::Option<::core::result::Result<
                ::composure::types::InteractionResponse,
                ::composure::command::HandlerError,
            >> {
                let (name, invocation) = invocation.route()?;
                #(#autocomplete_routes)*
                ::core::option::Option::None
            }
        }

        #route
    })
}
