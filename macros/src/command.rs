//! `#[command]`: keeps the function and adds a same-named empty struct implementing
//! `CommandDefinition` and `Command`.

use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::{
    FnArg, GenericArgument, Ident, ItemFn, LitInt, LitStr, Pat, PathArguments, ReturnType, Type,
    parse_quote, spanned::Spanned,
};

use crate::meta::{self, Settings};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    ChatInput,
    User,
    Message,
}

/// How the Command reaches the Adapter's State
enum State {
    /// No `Ctx` param, or `Ctx` without a State type: works with any State
    Generic,
    /// `Ctx<T>`: only with State `T`
    Concrete(Box<Type>),
}

struct OptionParam {
    ident: Ident,
    ty: Type,
    name: String,
    description: String,
    min: Option<f64>,
    max: Option<f64>,
    min_len: Option<i32>,
    max_len: Option<i32>,
    channel_types: Option<Vec<Ident>>,
    span: Span,
}

pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut settings = Settings::default();
    let mut kind = Kind::ChatInput;
    let parser = syn::meta::parser(|meta| {
        if settings.parse(&meta)? {
            Ok(())
        } else if meta.path.is_ident("user") {
            kind = Kind::User;
            Ok(())
        } else if meta.path.is_ident("message") {
            kind = Kind::Message;
            Ok(())
        } else {
            Err(meta.error(
                "unknown setting; expected one of name, desc, scope, permissions, contexts, integration_types, nsfw, user, message",
            ))
        }
    });
    syn::parse::Parser::parse2(parser, args)?;

    let mut function: ItemFn = syn::parse2(item)?;
    let ident = function.sig.ident.clone();
    let span = ident.span();

    if !function.sig.generics.params.is_empty() {
        return Err(syn::Error::new(
            function.sig.generics.span(),
            "commands cannot be generic; use `ctx: Ctx` to accept any State",
        ));
    }
    if matches!(function.sig.output, ReturnType::Default) {
        return Err(syn::Error::new(
            function.sig.span(),
            "commands must return a response, e.g. `-> String` or `-> Result<Embed, E>`",
        ));
    }

    let name = match &settings.name {
        Some(name) => (name.value(), name.span()),
        None => (ident.to_string(), span),
    };
    match kind {
        Kind::ChatInput => meta::validate_chat_name(&name.0, name.1)?,
        Kind::User | Kind::Message => meta::validate_context_name(&name.0, name.1)?,
    }
    let name = name.0;

    // User and message commands have no description on Discord's side
    let description = match kind {
        Kind::ChatInput => meta::description(&settings, &function.attrs, span)?,
        Kind::User | Kind::Message => String::new(),
    };

    // Split params into ctx and options/target, stripping `#[option]` attributes
    let mut state = State::Generic;
    let mut has_ctx = false;
    let mut options = Vec::new();
    for (index, input) in function.sig.inputs.iter_mut().enumerate() {
        let FnArg::Typed(param) = input else {
            return Err(syn::Error::new(input.span(), "commands cannot take `self`"));
        };

        if index == 0
            && let Some(ctx_state) = ctx_state(&param.ty)
        {
            has_ctx = true;
            // Spell out the lifetime: async fns reject an elided one in `Ctx<T>`
            let args: syn::AngleBracketedGenericArguments = match &ctx_state {
                Some(ty) => parse_quote!(<'_, #ty>),
                None => parse_quote!(<'_, __S>),
            };
            if let Type::Path(path) = &mut *param.ty
                && let Some(segment) = path.path.segments.last_mut()
            {
                segment.arguments = PathArguments::AngleBracketed(args);
            }
            if let Some(ty) = ctx_state {
                state = State::Concrete(Box::new(ty));
            }
            continue;
        }

        let Pat::Ident(pat) = &*param.pat else {
            return Err(syn::Error::new(
                param.pat.span(),
                "option parameters must be plain names",
            ));
        };
        let option = parse_option(pat.ident.clone(), (*param.ty).clone(), &param.attrs, kind)?;
        param.attrs.retain(|attr| !attr.path().is_ident("option"));
        options.push(option);
    }

    match kind {
        Kind::ChatInput if options.len() > 25 => {
            return Err(syn::Error::new(
                span,
                "commands can have at most 25 options",
            ));
        }
        Kind::User | Kind::Message if options.len() != 1 => {
            return Err(syn::Error::new(
                span,
                "user and message commands take exactly one target parameter after `ctx`",
            ));
        }
        _ => {}
    }
    for (i, option) in options.iter().enumerate() {
        if options[..i].iter().any(|o| o.name == option.name) {
            return Err(syn::Error::new(
                option.span,
                format!("duplicate option name `{}`", option.name),
            ));
        }
    }

    let vis = &function.vis;
    let docs: Vec<_> = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .collect();
    if matches!(state, State::Generic) && has_ctx {
        function.sig.generics = parse_quote!(<__S>);
    }

    let settings_meta = settings.meta_tokens(&name, &description);
    let scope = settings.scope_tokens();
    let (kind_tokens, definition) = match kind {
        Kind::ChatInput => (
            quote!(ChatInput),
            quote! { Self::__meta().chat_input(Self::__options()) },
        ),
        Kind::User => (quote!(User), quote! { Self::__meta().user() }),
        Kind::Message => (quote!(Message), quote! { Self::__meta().message() }),
    };

    // User and message targets are not Options
    let option_definitions: Vec<_> = match kind {
        Kind::ChatInput => options.iter().map(option_definition).collect(),
        Kind::User | Kind::Message => Vec::new(),
    };
    let checks = checks(&options, kind);

    let (impl_generics, state_ty) = match &state {
        State::Generic => (quote!(<__S>), quote!(__S)),
        State::Concrete(ty) => (quote!(), quote!(#ty)),
    };

    let extracts = options.iter().map(|option| {
        let OptionParam { ident, ty, name, .. } = option;
        match kind {
            Kind::ChatInput => quote! {
                let #ident = <#ty as ::composure::command::OptionType>::extract(
                    invocation.option(#name),
                    invocation.data(),
                )
                .map_err(|e| ::composure::command::HandlerError::invalid_option(#name, e))?;
            },
            Kind::User | Kind::Message => quote! {
                let #ident = <#ty as ::composure::command::CommandTarget>::extract(invocation.data())
                    .map_err(|e| ::composure::command::HandlerError::invalid_option("target", e))?;
            },
        }
    });
    let ctx_arg = has_ctx.then(|| quote!(invocation.ctx(),));
    let args = options.iter().map(|option| &option.ident);
    let call = if function.sig.asyncness.is_some() {
        quote! { #ident(#ctx_arg #(#args),*).await }
    } else {
        quote! { #ident(#ctx_arg #(#args),*) }
    };

    let subcommand = (kind == Kind::ChatInput).then(|| {
        quote! {
            impl ::composure::command::SubcommandDefinition for #ident {
                const DEPTH: u8 = 0;

                fn option_definition() -> ::composure::definition::ApplicationCommandOption {
                    Self::__meta().subcommand(Self::__options())
                }
            }
        }
    });

    // A braced struct lives only in the type namespace, so the function keeps its name
    Ok(quote! {
        #function

        #(#docs)*
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, Default)]
        #vis struct #ident {}

        impl #ident {
            #[doc(hidden)]
            fn __meta() -> ::composure::command::CommandMeta {
                #settings_meta
            }

            #[doc(hidden)]
            #[allow(unused_mut)]
            fn __options() -> ::std::vec::Vec<::composure::definition::ApplicationCommandOption> {
                ::std::vec![#(#option_definitions),*]
            }
        }

        #checks

        impl ::composure::command::CommandDefinition for #ident {
            const NAME: &'static str = #name;
            const KIND: ::composure::models::ApplicationCommandType =
                ::composure::models::ApplicationCommandType::#kind_tokens;
            const SCOPE: ::core::option::Option<&'static str> = #scope;

            fn definition() -> ::composure::definition::ApplicationCommand {
                #definition
            }
        }

        #subcommand

        impl #impl_generics ::composure::command::Command<#state_ty> for #ident {
            async fn invoke(
                invocation: ::composure::command::Invocation<'_, #state_ty>,
            ) -> ::core::result::Result<
                ::composure::models::InteractionResponse,
                ::composure::command::HandlerError,
            > {
                #(#extracts)*
                ::composure::command::CommandOutput::into_result(#call)
            }
        }
    })
}

/// `Some(state)` if `ty` is `Ctx`: `Some(None)` for `Ctx`/`Ctx<'_>`, `Some(Some(T))` for `Ctx<T>`
fn ctx_state(ty: &Type) -> Option<Option<Type>> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    if segment.ident != "Ctx" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return Some(None);
    };
    Some(args.args.iter().find_map(|arg| match arg {
        GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    }))
}

fn parse_option(
    ident: Ident,
    ty: Type,
    attrs: &[syn::Attribute],
    kind: Kind,
) -> syn::Result<OptionParam> {
    let span = ident.span();
    let mut option = OptionParam {
        name: ident.to_string(),
        ident,
        ty,
        description: String::new(),
        min: None,
        max: None,
        min_len: None,
        max_len: None,
        channel_types: None,
        span,
    };
    let mut desc: Option<LitStr> = None;
    let mut has_attr = false;

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("option")) {
        has_attr = true;
        if kind != Kind::ChatInput {
            return Err(syn::Error::new(
                attr.span(),
                "`#[option]` is only for chat-input commands; user and message targets have no settings",
            ));
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("desc") || meta.path.is_ident("description") {
                desc = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("name") || meta.path.is_ident("rename") {
                let name: LitStr = meta.value()?.parse()?;
                option.name = name.value();
                option.span = name.span();
            } else if meta.path.is_ident("min") {
                option.min = Some(meta::number(&meta.value()?.parse()?)?);
            } else if meta.path.is_ident("max") {
                option.max = Some(meta::number(&meta.value()?.parse()?)?);
            } else if meta.path.is_ident("min_len") {
                option.min_len = Some(length(&meta.value()?.parse()?, 0)?);
            } else if meta.path.is_ident("max_len") {
                option.max_len = Some(length(&meta.value()?.parse()?, 1)?);
            } else if meta.path.is_ident("channel_types") {
                option.channel_types = Some(meta::ident_list(&meta)?);
            } else {
                return Err(meta.error(
                    "unknown option setting; expected one of desc, name, min, max, min_len, max_len, channel_types",
                ));
            }
            Ok(())
        })?;
    }

    if kind == Kind::ChatInput {
        meta::validate_chat_name(&option.name, option.span)?;
        let desc = desc.ok_or_else(|| {
            syn::Error::new(
                span,
                if has_attr {
                    "missing option description: add `desc = \"...\"`"
                } else {
                    "missing option description: add `#[option(desc = \"...\")]`"
                },
            )
        })?;
        meta::validate_description(&desc.value(), desc.span())?;
        option.description = desc.value();
    }
    if let (Some(min), Some(max)) = (option.min, option.max)
        && min > max
    {
        return Err(syn::Error::new(span, "`min` is greater than `max`"));
    }
    if let (Some(min), Some(max)) = (option.min_len, option.max_len)
        && min > max
    {
        return Err(syn::Error::new(span, "`min_len` is greater than `max_len`"));
    }
    Ok(option)
}

/// String length bound: `min..=6000`
fn length(lit: &LitInt, min: i32) -> syn::Result<i32> {
    let value: i32 = lit.base10_parse()?;
    if !(min..=6000).contains(&value) {
        return Err(syn::Error::new(
            lit.span(),
            format!("length must be between {min} and 6000"),
        ));
    }
    Ok(value)
}

fn option_definition(option: &OptionParam) -> TokenStream {
    let OptionParam {
        ty,
        name,
        description,
        ..
    } = option;
    let mut fields = Vec::new();
    if let Some(min) = option.min {
        let min = Literal::f64_suffixed(min);
        fields.push(quote!(min_value: ::core::option::Option::Some(#min),));
    }
    if let Some(max) = option.max {
        let max = Literal::f64_suffixed(max);
        fields.push(quote!(max_value: ::core::option::Option::Some(#max),));
    }
    if let Some(min_len) = option.min_len {
        fields.push(quote!(min_length: ::core::option::Option::Some(#min_len),));
    }
    if let Some(max_len) = option.max_len {
        fields.push(quote!(max_length: ::core::option::Option::Some(#max_len),));
    }
    if let Some(types) = &option.channel_types {
        fields.push(quote! {
            channel_types: ::core::option::Option::Some(
                &[#(::composure::models::ChannelType::#types),*]
            ),
        });
    }
    quote! {
        <#ty as ::composure::command::OptionType>::definition(
            &::composure::command::OptionMeta {
                #(#fields)*
                ..::composure::command::OptionMeta::new(#name, #description)
            },
            <#ty as ::composure::command::OptionType>::REQUIRED,
        )
    }
}

/// Compile-time checks that need the option types: settings that fit the type, bounds inside
/// the type's range, required before optional, and targets matching the command kind
fn checks(options: &[OptionParam], kind: Kind) -> TokenStream {
    if kind != Kind::ChatInput {
        let ty = &options[0].ty;
        let (expected, message) = match kind {
            Kind::User => (
                quote!(User),
                "a user command's target must be `User` or `ResolvedMember`",
            ),
            _ => (
                quote!(Message),
                "a message command's target must be `Message`",
            ),
        };
        return quote! {
            const _: () = ::core::assert!(
                ::core::matches!(
                    <#ty as ::composure::command::CommandTarget>::KIND,
                    ::composure::models::ApplicationCommandType::#expected
                ),
                #message
            );
        };
    }

    let mut checks = Vec::new();
    for option in options {
        let ty = &option.ty;
        let name = &option.name;
        let option_type = quote!(<#ty as ::composure::command::OptionType>::OPTION_TYPE);
        if option.min.is_some() || option.max.is_some() {
            let message =
                format!("option `{name}`: `min`/`max` only apply to integer and number options");
            checks.push(quote! {
                ::core::assert!(#option_type == 4 || #option_type == 10, #message);
            });
        }
        if let Some(min) = option.min {
            let min = Literal::f64_suffixed(min);
            let message = format!("option `{name}`: `min` is below the range of its type");
            checks.push(quote! {
                if let ::core::option::Option::Some(type_min) =
                    <#ty as ::composure::command::OptionType>::MIN_VALUE
                {
                    ::core::assert!(#min >= type_min, #message);
                }
            });
        }
        if let Some(max) = option.max {
            let max = Literal::f64_suffixed(max);
            let message = format!("option `{name}`: `max` is above the range of its type");
            checks.push(quote! {
                if let ::core::option::Option::Some(type_max) =
                    <#ty as ::composure::command::OptionType>::MAX_VALUE
                {
                    ::core::assert!(#max <= type_max, #message);
                }
            });
        }
        if option.min_len.is_some() || option.max_len.is_some() {
            let message =
                format!("option `{name}`: `min_len`/`max_len` only apply to string options");
            checks.push(quote! { ::core::assert!(#option_type == 3, #message); });
        }
        if option.channel_types.is_some() {
            let message =
                format!("option `{name}`: `channel_types` only applies to channel options");
            checks.push(quote! { ::core::assert!(#option_type == 7, #message); });
        }
    }

    if options.len() > 1 {
        let types = options.iter().map(|option| &option.ty);
        checks.push(quote! {
            let required = [#(<#types as ::composure::command::OptionType>::REQUIRED),*];
            let mut i = 1;
            while i < required.len() {
                ::core::assert!(
                    required[i - 1] || !required[i],
                    "required options must come before optional ones",
                );
                i += 1;
            }
        });
    }

    if checks.is_empty() {
        return quote!();
    }
    quote! {
        const _: () = {
            #(#checks)*
        };
    }
}
