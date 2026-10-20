//! `#[app(handlers(..), theme = .., events = ..)]`: makes a unit struct an App routing to the
//! Commands, Groups, Component and Modal Handlers, Views and Apps it lists (ADR 0013). Also the
//! `Route` impls the other attributes add to what they define.

use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Expr, Fields, ItemStruct, Path};

pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut handlers: Option<Vec<Path>> = None;
    let mut theme: Option<Expr> = None;
    let mut events: Option<Path> = None;
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("handlers") {
            if handlers.is_some() {
                return Err(meta.error("`handlers` is set twice"));
            }
            let mut paths = Vec::new();
            meta.parse_nested_meta(|item| {
                paths.push(item.path.clone());
                Ok(())
            })?;
            handlers = Some(paths);
        } else if meta.path.is_ident("theme") {
            if theme.is_some() {
                return Err(meta.error("`theme` is set twice"));
            }
            theme = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("events") {
            if events.is_some() {
                return Err(meta.error("`events` is set twice"));
            }
            events = Some(meta.value()?.parse()?);
        } else {
            return Err(meta.error("unknown setting; expected one of handlers, theme, events"));
        }
        Ok(())
    });
    syn::parse::Parser::parse2(parser, args)?;
    let handlers = handlers.unwrap_or_default();

    let item: ItemStruct = syn::parse2(item)?;
    let ident = &item.ident;
    if !matches!(item.fields, Fields::Unit) || !item.generics.params.is_empty() {
        return Err(syn::Error::new(
            ident.span(),
            "apps must be unit structs: `struct Bot;`",
        ));
    }
    let attrs = &item.attrs;
    let vis = &item.vis;

    let c = quote!(::composure::command);
    let t = quote!(::composure::types);
    let routed = quote!(#c::__Routed);
    let theme_ty = quote!(::core::option::Option<&'static #c::Theme>);

    let routes = handlers
        .iter()
        .map(|h| quote_spanned!(h.span()=> <#h as #c::RouteDefinition>::ROUTE));
    let bounds = handlers
        .iter()
        .map(|h| quote_spanned!(h.span()=> #h: #c::Route<__S>,));
    let events_bound = events
        .as_ref()
        .map(|e| quote_spanned!(e.span()=> #e: #c::EventHandler<__S>,));
    let definitions = handlers
        .iter()
        .map(|h| quote_spanned!(h.span()=> <#h as #c::RouteDefinition>::definitions(into);));
    let has_events = events.is_some();
    let own_theme = theme.as_ref().map(|theme| {
        quote! {
            const THEME: #theme_ty = ::core::option::Option::Some(&#theme);
        }
    });
    // Without its own theme, an App passes the outer one through and leaves responses alone
    let (inherit, themed) = if theme.is_some() {
        (
            quote!(let theme = <Self as #c::RouteDefinition>::THEME.or(theme);),
            quote!(#c::__themed(routed, <Self as #c::RouteDefinition>::THEME)),
        )
    } else {
        (quote!(), quote!(routed))
    };
    // Children that cannot answer this kind of Interaction are skipped at compile time, so their
    // (always `None`) futures are not part of the App's
    let method = |name: TokenStream, interaction: TokenStream, answers: TokenStream| {
        let calls = handlers.iter().map(|h| {
            quote_spanned! {h.span()=>
                if const { <#h as #c::RouteDefinition>::ROUTE.#answers() }
                    && let ::core::option::Option::Some(result) =
                        <#h as #c::Route<__S>>::#name(interaction, state, theme).await
                {
                    break 'routed ::core::option::Option::Some(result);
                }
            }
        });
        quote! {
            async fn #name(
                interaction: &::std::sync::Arc<#t::#interaction>,
                state: &__S,
                theme: #theme_ty,
            ) -> #routed {
                #inherit
                #[allow(unused_labels)]
                let routed = 'routed: {
                    #(#calls)*
                    ::core::option::Option::None
                };
                #themed
            }
        }
    };
    let dispatch = method(
        quote!(dispatch),
        quote!(ApplicationCommandInteraction),
        quote!(has_commands),
    );
    let dispatch_autocomplete = method(
        quote!(dispatch_autocomplete),
        quote!(ApplicationCommandInteraction),
        quote!(has_commands),
    );
    let dispatch_component = method(
        quote!(dispatch_component),
        quote!(MessageComponentInteraction),
        quote!(has_components),
    );
    let dispatch_modal = method(
        quote!(dispatch_modal),
        quote!(ModalSubmitInteraction),
        quote!(has_modals),
    );
    // At most one App in the tree sets `events` (checked below), so the first that has it wins
    let dispatch_event = match &events {
        Some(events) => quote! {
            #c::EventHandler::<__S>::handle(&#events, ctx, event).await
        },
        None => {
            let calls = handlers.iter().map(|h| {
                quote_spanned! {h.span()=>
                    if const { <#h as #c::RouteDefinition>::ROUTE.has_events() } {
                        return <#h as #c::Route<__S>>::dispatch_event(ctx, event).await;
                    }
                }
            });
            quote! {
            #(#calls)*
            let _ = (ctx, event);
            ::core::result::Result::Ok(())
            }
        }
    };

    let check = quote_spanned! {ident.span()=>
        const _: () = #c::check_app(&<#ident as #c::RouteDefinition>::ROUTE);
    };

    Ok(quote! {
        #(#attrs)*
        #[derive(Debug, Clone, Copy, Default)]
        #vis struct #ident;

        #check

        impl #ident {
            /// Sends components, modals, autocomplete, and commands this App does not handle to
            /// `fallback`
            pub fn with_fallback<F>(self, fallback: F) -> #c::WithFallback<Self, F> {
                #c::WithFallback::new(self, fallback)
            }
        }

        impl #c::RouteDefinition for #ident {
            const ROUTE: #c::RouteInfo = #c::RouteInfo::App {
                routes: &[#(#routes),*],
                events: #has_events,
            };

            #own_theme

            fn definitions(into: &mut ::std::vec::Vec<#c::ScopedDefinition>) {
                let _ = &into;
                #(#definitions)*
            }
        }

        impl<__S: ::core::marker::Sync> #c::Route<__S> for #ident
        where
            #(#bounds)*
            #events_bound
        {
            #dispatch
            #dispatch_autocomplete
            #dispatch_component
            #dispatch_modal

            async fn dispatch_event(
                ctx: #c::EventCtx<'_, __S>,
                event: #t::Event,
            ) -> ::core::result::Result<(), #c::HandlerError> {
                #dispatch_event
            }
        }

        impl<__S: ::core::marker::Sync> #c::InteractionHandler<__S> for #ident
        where
            #ident: #c::Route<__S>,
        {
            fn theme(&self) -> #theme_ty {
                <Self as #c::RouteDefinition>::THEME
            }

            async fn command(
                &self,
                command: #t::ApplicationCommandInteraction,
                state: &__S,
            ) -> ::core::result::Result<#t::InteractionResponse, #c::HandlerError> {
                let command = ::std::sync::Arc::new(command);
                match <Self as #c::Route<__S>>::dispatch(&command, state, ::core::option::Option::None).await {
                    ::core::option::Option::Some(result) => result,
                    ::core::option::Option::None => ::core::result::Result::Err(
                        #c::HandlerError::unknown_command(&command.data.name),
                    ),
                }
            }

            async fn autocomplete(
                &self,
                autocomplete: #t::ApplicationCommandInteraction,
                state: &__S,
            ) -> ::core::result::Result<#t::InteractionResponse, #c::HandlerError> {
                let autocomplete = ::std::sync::Arc::new(autocomplete);
                match <Self as #c::Route<__S>>::dispatch_autocomplete(&autocomplete, state, ::core::option::Option::None).await {
                    ::core::option::Option::Some(result) => result,
                    ::core::option::Option::None => {
                        #c::InteractionHandler::<__S>::autocomplete(
                            &#c::NoHandler, #c::__unshare(autocomplete), state,
                        )
                        .await
                    }
                }
            }

            async fn component(
                &self,
                component: #t::MessageComponentInteraction,
                state: &__S,
            ) -> ::core::result::Result<#t::InteractionResponse, #c::HandlerError> {
                let component = ::std::sync::Arc::new(component);
                match <Self as #c::Route<__S>>::dispatch_component(&component, state, ::core::option::Option::None).await {
                    ::core::option::Option::Some(result) => result,
                    ::core::option::Option::None => {
                        #c::InteractionHandler::<__S>::component(
                            &#c::NoHandler, #c::__unshare(component), state,
                        )
                        .await
                    }
                }
            }

            async fn modal_submit(
                &self,
                modal: #t::ModalSubmitInteraction,
                state: &__S,
            ) -> ::core::result::Result<#t::InteractionResponse, #c::HandlerError> {
                let modal = ::std::sync::Arc::new(modal);
                match <Self as #c::Route<__S>>::dispatch_modal(&modal, state, ::core::option::Option::None).await {
                    ::core::option::Option::Some(result) => result,
                    ::core::option::Option::None => {
                        #c::InteractionHandler::<__S>::modal_submit(
                            &#c::NoHandler, #c::__unshare(modal), state,
                        )
                        .await
                    }
                }
            }
        }

        impl<__S: ::core::marker::Sync> #c::EventHandler<__S> for #ident
        where
            #ident: #c::Route<__S>,
        {
            async fn handle(
                &self,
                ctx: #c::EventCtx<'_, __S>,
                event: #t::Event,
            ) -> ::core::result::Result<(), #c::HandlerError> {
                <Self as #c::Route<__S>>::dispatch_event(ctx, event).await
            }
        }

        impl #c::CommandSetDefinition for #ident {
            fn definitions() -> ::std::vec::Vec<#c::ScopedDefinition> {
                let mut definitions = ::std::vec::Vec::new();
                <Self as #c::RouteDefinition>::definitions(&mut definitions);
                definitions
            }
        }
    })
}

/// One `Route` dispatch method forwarding to a `command::__dispatch_*` helper
fn forward(name: &str, interaction: &str, helper: &str) -> TokenStream {
    let c = quote!(::composure::command);
    let name = Ident::new(name, proc_macro2::Span::call_site());
    let interaction = Ident::new(interaction, proc_macro2::Span::call_site());
    let helper = Ident::new(helper, proc_macro2::Span::call_site());
    quote! {
        fn #name(
            interaction: &::std::sync::Arc<::composure::types::#interaction>,
            state: &__S,
            theme: ::core::option::Option<&'static #c::Theme>,
        ) -> impl ::core::future::Future<Output = #c::__Routed> + ::core::marker::Send {
            #c::#helper::<Self, __S>(interaction, state, theme)
        }
    }
}

/// `RouteDefinition` and `Route` for a `#[command]` or `#[group]`
pub fn command_route(ident: &Ident) -> TokenStream {
    let c = quote!(::composure::command);
    let dispatch = forward(
        "dispatch",
        "ApplicationCommandInteraction",
        "__dispatch_command",
    );
    let autocomplete = forward(
        "dispatch_autocomplete",
        "ApplicationCommandInteraction",
        "__dispatch_autocomplete",
    );
    quote! {
        impl #c::RouteDefinition for #ident {
            const ROUTE: #c::RouteInfo = #c::RouteInfo::command::<Self>();

            fn definitions(into: &mut ::std::vec::Vec<#c::ScopedDefinition>) {
                into.push(#c::ScopedDefinition::of::<Self>());
            }
        }

        impl<__S: ::core::marker::Sync> #c::Route<__S> for #ident
        where
            #ident: #c::Command<__S>,
        {
            #dispatch
            #autocomplete
        }
    }
}

/// `RouteDefinition` and `Route` for a `#[component]` (`modal: false`) or `#[modal]`
pub fn handler_route(ident: &Ident, modal: bool) -> TokenStream {
    let c = quote!(::composure::command);
    let (variant, definition, handler, method) = if modal {
        (
            quote!(Modal),
            quote!(ModalDefinition),
            quote!(ModalHandler),
            forward(
                "dispatch_modal",
                "ModalSubmitInteraction",
                "__dispatch_modal",
            ),
        )
    } else {
        (
            quote!(Component),
            quote!(ComponentDefinition),
            quote!(ComponentHandler),
            forward(
                "dispatch_component",
                "MessageComponentInteraction",
                "__dispatch_component",
            ),
        )
    };
    quote! {
        impl #c::RouteDefinition for #ident {
            const ROUTE: #c::RouteInfo = #c::RouteInfo::#variant {
                prefix: <Self as #c::#definition>::PREFIX,
            };
        }

        impl<__S: ::core::marker::Sync> #c::Route<__S> for #ident
        where
            #ident: #c::#handler<__S>,
        {
            #method
        }
    }
}

/// `RouteDefinition` and `Route` for a `#[view]`
pub fn view_route(ident: &Ident) -> TokenStream {
    let c = quote!(::composure::command);
    let component = forward(
        "dispatch_component",
        "MessageComponentInteraction",
        "__dispatch_view_component",
    );
    let modal = forward(
        "dispatch_modal",
        "ModalSubmitInteraction",
        "__dispatch_view_modal",
    );
    quote! {
        impl #c::RouteDefinition for #ident {
            const ROUTE: #c::RouteInfo =
                #c::RouteInfo::View(<Self as #c::ViewDefinition>::INFO);
        }

        impl<__S: ::core::marker::Sync> #c::Route<__S> for #ident
        where
            #ident: #c::ViewHandler<__S>,
        {
            #component
            #modal
        }
    }
}
