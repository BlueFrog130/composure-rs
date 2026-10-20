//! `#[command]`: keeps the function and adds a same-named empty struct implementing
//! `CommandDefinition` and `Command`.

use proc_macro2::{Literal, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{
    FnArg, GenericArgument, GenericParam, Ident, ItemFn, LitInt, LitStr, Pat, PathArguments,
    ReturnType, Token, Type, parse::Parser, parse_quote, spanned::Spanned,
};

use crate::meta::{self, Settings};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    ChatInput,
    User,
    Message,
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
    autocomplete: Option<syn::Path>,
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
    parser.parse2(args)?;

    let mut function: ItemFn = syn::parse2(item)?;
    let ident = function.sig.ident.clone();
    let span = ident.span();

    let state_param = state_param(&function)?;
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
    let mut ctx_state_ty: Option<Type> = None;
    let mut has_ctx = false;
    let mut options = Vec::new();
    for (index, input) in function.sig.inputs.iter_mut().enumerate() {
        let FnArg::Typed(param) = input else {
            return Err(syn::Error::new(input.span(), "commands cannot take `self`"));
        };

        if let Some(state) = ctx_state(&param.ty) {
            if index != 0 {
                return Err(syn::Error::new(
                    param.ty.span(),
                    "`ctx` must be the first parameter",
                ));
            }
            has_ctx = true;
            ctx_state_ty = state;
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

    // The State the Command runs with: a concrete type, or a type parameter of the function
    // (`Ctx` and `Ctx<impl Bound>` add one named `__S`)
    let state: Type = match ctx_state_ty {
        None if has_ctx => {
            if let Some(param) = &state_param {
                return Err(syn::Error::new(
                    param.span(),
                    "use the type parameter as the State: `ctx: Ctx<S>`",
                ));
            }
            function.sig.generics.params.push(parse_quote!(__S));
            parse_quote!(__S)
        }
        Some(Type::ImplTrait(bounds)) => {
            let bounds = bounds.bounds;
            function
                .sig
                .generics
                .params
                .push(parse_quote!(__S: #bounds));
            parse_quote!(__S)
        }
        Some(ty) => {
            if let Some(param) = &state_param
                && !matches!(&ty, Type::Path(path) if path.path.is_ident(param))
            {
                return Err(syn::Error::new(
                    ty.span(),
                    format!("commands can only be generic over their State: `ctx: Ctx<{param}>`"),
                ));
            }
            ty
        }
        None => {
            if let Some(param) = &state_param {
                return Err(syn::Error::new(
                    param.span(),
                    format!(
                        "commands can only be generic over their State: add `ctx: Ctx<{param}>`"
                    ),
                ));
            }
            parse_quote!(__S)
        }
    };
    if has_ctx
        && let Some(FnArg::Typed(param)) = function.sig.inputs.first_mut()
        && let Type::Path(path) = &mut *param.ty
        && let Some(segment) = path.path.segments.last_mut()
    {
        // Spell out the lifetime: async fns reject an elided one in `Ctx<T>`
        segment.arguments = PathArguments::AngleBracketed(parse_quote!(<'_, #state>));
    }
    let generic = has_ctx && function.sig.generics.type_params().next().is_some();

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

    // Autocomplete Handlers must accept the Command's State
    let autocompletes: Vec<_> = options
        .iter()
        .filter_map(|o| Some((&o.name, o.autocomplete.as_ref()?)))
        .collect();
    let mut predicates: Vec<TokenStream> = function
        .sig
        .generics
        .where_clause
        .iter()
        .flat_map(|clause| clause.predicates.iter().map(|p| quote!(#p)))
        .collect();
    predicates.extend(
        autocompletes
            .iter()
            .map(|(_, handler)| quote!(#handler: ::composure::command::Autocomplete<#state>)),
    );
    // Command futures are `Send`, and they hold the Invocation (so `&State`) across `.await`
    predicates.push(quote!(#state: ::core::marker::Sync));
    let where_clause = quote!(where #(#predicates),*);
    let impl_generics = if generic {
        let (impl_generics, _, _) = function.sig.generics.split_for_impl();
        quote!(#impl_generics)
    } else if has_ctx {
        quote!()
    } else {
        quote!(<__S>)
    };
    let autocomplete = (!autocompletes.is_empty()).then(|| {
        let routes = autocompletes.iter().map(|(name, handler)| {
            quote! {
                if focused.name == #name {
                    return ::core::option::Option::Some(
                        <#handler as ::composure::command::Autocomplete<#state>>::complete(
                            invocation.ctx(),
                            &focused.value,
                        )
                        .await,
                    );
                }
            }
        });
        quote! {
            async fn autocomplete(
                invocation: ::composure::command::Invocation<'_, #state>,
            ) -> ::core::option::Option<::core::result::Result<
                ::composure::types::InteractionResponse,
                ::composure::command::HandlerError,
            >> {
                let focused = invocation.focused()?;
                #(#routes)*
                ::core::option::Option::None
            }
        }
    });

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

    let route = crate::app::command_route(&ident);

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
            const KIND: ::composure::types::ApplicationCommandType =
                ::composure::types::ApplicationCommandType::#kind_tokens;
            const SCOPE: ::core::option::Option<&'static str> = #scope;

            fn definition() -> ::composure::definition::ApplicationCommand {
                #definition
            }
        }

        #subcommand

        impl #impl_generics ::composure::command::Command<#state> for #ident #where_clause {
            async fn invoke(
                invocation: ::composure::command::Invocation<'_, #state>,
            ) -> ::core::result::Result<
                ::composure::types::InteractionResponse,
                ::composure::command::HandlerError,
            > {
                #(#extracts)*
                ::composure::command::CommandOutput::into_result(#call)
            }

            #autocomplete
        }

        #route
    })
}

/// The function's one type parameter, which must be its State; lifetimes and consts are rejected
fn state_param(function: &ItemFn) -> syn::Result<Option<Ident>> {
    let generics = &function.sig.generics;
    let mut param = None;
    for generic in &generics.params {
        match generic {
            GenericParam::Type(ty) if param.is_none() => param = Some(ty.ident.clone()),
            _ => {
                return Err(syn::Error::new(
                    generic.span(),
                    "commands can only be generic over their State: `fn f<S: Bound>(ctx: Ctx<S>)`",
                ));
            }
        }
    }
    Ok(param)
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

/// `#[option("Description", setting = value, ...)]`
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
        autocomplete: None,
        span,
    };
    let mut desc: Option<LitStr> = None;

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("option")) {
        if kind != Kind::ChatInput {
            return Err(syn::Error::new(
                attr.span(),
                "`#[option]` is only for chat-input commands; user and message targets have no settings",
            ));
        }
        if matches!(attr.meta, syn::Meta::Path(_)) {
            continue;
        }
        attr.parse_args_with(|input: syn::parse::ParseStream| {
            if input.peek(LitStr) {
                desc = Some(input.parse()?);
                if input.is_empty() {
                    return Ok(());
                }
                input.parse::<Token![,]>()?;
            }
            let settings = syn::meta::parser(|meta| {
                if meta.path.is_ident("name") {
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
                } else if meta.path.is_ident("autocomplete") {
                    option.autocomplete = Some(meta.value()?.parse()?);
                } else {
                    return Err(meta.error(
                        "unknown option setting; expected a description first, then name, min, max, min_len, max_len, channel_types, autocomplete",
                    ));
                }
                Ok(())
            });
            settings.parse2(input.parse()?)
        })?;
    }

    if kind == Kind::ChatInput {
        meta::validate_chat_name(&option.name, option.span)?;
        let desc = desc.ok_or_else(|| {
            syn::Error::new(span, "missing option description: add `#[option(\"...\")]`")
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
    if option.autocomplete.is_some() {
        fields.push(quote!(autocomplete: true,));
    }
    if let Some(types) = &option.channel_types {
        fields.push(quote! {
            channel_types: ::core::option::Option::Some(
                &[#(::composure::types::ChannelType::#types),*]
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
/// the type's range, required before optional, and targets matching the command kind. Each is
/// spanned to its parameter's type, so the error points there.
fn checks(options: &[OptionParam], kind: Kind) -> TokenStream {
    if kind != Kind::ChatInput {
        let ty = &options[0].ty;
        let (expected, message) = match kind {
            Kind::User => (
                quote!(User),
                "a user command's target must be `User`, `&User`, or `ResolvedMember`",
            ),
            _ => (
                quote!(Message),
                "a message command's target must be `Message` or `&Message`",
            ),
        };
        return quote_spanned! {ty.span()=>
            const _: () = ::core::assert!(
                ::core::matches!(
                    <#ty as ::composure::command::CommandTarget>::KIND,
                    ::composure::types::ApplicationCommandType::#expected
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
        let mut check = |message: String, condition: TokenStream| {
            checks.push(quote_spanned! {ty.span()=>
                ::core::assert!(#condition, #message);
            });
        };
        if option.min.is_some() || option.max.is_some() {
            check(
                format!("option `{name}`: `min`/`max` only apply to integer and number options"),
                quote!(#option_type == 4 || #option_type == 10),
            );
        }
        if [option.min, option.max]
            .into_iter()
            .flatten()
            .any(|bound| bound.fract() != 0.0)
        {
            check(
                format!(
                    "option `{name}`: fractional `min`/`max` only apply to number (`f64`) options"
                ),
                quote!(#option_type == 10),
            );
        }
        if let Some(min) = option.min {
            let min = Literal::f64_suffixed(min);
            check(
                format!("option `{name}`: `min` is below the range of its type"),
                quote! {
                    match <#ty as ::composure::command::OptionType>::MIN_VALUE {
                        ::core::option::Option::Some(type_min) => #min >= type_min,
                        ::core::option::Option::None => true,
                    }
                },
            );
        }
        if let Some(max) = option.max {
            let max = Literal::f64_suffixed(max);
            check(
                format!("option `{name}`: `max` is above the range of its type"),
                quote! {
                    match <#ty as ::composure::command::OptionType>::MAX_VALUE {
                        ::core::option::Option::Some(type_max) => #max <= type_max,
                        ::core::option::Option::None => true,
                    }
                },
            );
        }
        if option.min_len.is_some() || option.max_len.is_some() {
            check(
                format!("option `{name}`: `min_len`/`max_len` only apply to string options"),
                quote!(#option_type == 3),
            );
        }
        if option.channel_types.is_some() {
            check(
                format!("option `{name}`: `channel_types` only applies to channel options"),
                quote!(#option_type == 7),
            );
        }
        if option.autocomplete.is_some() {
            check(
                format!(
                    "option `{name}`: `autocomplete` only applies to string, integer, and number options without fixed choices"
                ),
                quote! {
                    (#option_type == 3 || #option_type == 4 || #option_type == 10)
                        && !<#ty as ::composure::command::OptionType>::HAS_CHOICES
                },
            );
        }
    }

    // Each required option must not follow an optional one
    for pair in options.windows(2) {
        let (previous, ty) = (&pair[0].ty, &pair[1].ty);
        let message = format!(
            "required option `{}` must come before optional ones",
            pair[1].name
        );
        checks.push(quote_spanned! {ty.span()=>
            ::core::assert!(
                <#previous as ::composure::command::OptionType>::REQUIRED
                    || !<#ty as ::composure::command::OptionType>::REQUIRED,
                #message,
            );
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
