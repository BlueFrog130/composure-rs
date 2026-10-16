//! `#[component("prefix")]` and `#[modal("prefix")]`: keep the function and add a same-named
//! empty struct implementing `ComponentHandler` / `ModalHandler`, with `custom_id(..)` (and
//! `show(..)` for modals) built from the handler's state parameters.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    FnArg, GenericArgument, GenericParam, Ident, ItemFn, LitStr, Pat, PathArguments, ReturnType,
    Type, parse_quote, spanned::Spanned,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Component,
    Modal,
}

impl Kind {
    fn ctx_name(self) -> &'static str {
        match self {
            Kind::Component => "ComponentCtx",
            Kind::Modal => "ModalCtx",
        }
    }

    fn attribute(self) -> &'static str {
        match self {
            Kind::Component => "component",
            Kind::Modal => "modal",
        }
    }
}

const MAX_PREFIX_LEN: usize = 100;

pub fn expand(kind: Kind, args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let prefix: LitStr = syn::parse2(args).map_err(|e| {
        syn::Error::new(
            e.span(),
            format!(
                "expected the custom_id prefix: `#[{}(\"prefix\")]`",
                kind.attribute()
            ),
        )
    })?;
    validate_prefix(&prefix)?;

    let mut function: ItemFn = syn::parse2(item)?;
    let ident = function.sig.ident.clone();
    if matches!(function.sig.output, ReturnType::Default) {
        return Err(syn::Error::new(
            function.sig.span(),
            "handlers must return a response, e.g. `-> String` or `-> Update<String>`",
        ));
    }
    let state_param = state_param(&function)?;

    // ctx, then (modals) the form, then the custom_id state
    let mut ctx: Option<Option<Type>> = None;
    let mut form: Option<(Ident, Type)> = None;
    let mut state_args: Vec<(Ident, Type)> = Vec::new();
    for (index, input) in function.sig.inputs.iter().enumerate() {
        let FnArg::Typed(param) = input else {
            return Err(syn::Error::new(input.span(), "handlers cannot take `self`"));
        };
        if let Some(state) = ctx_state(&param.ty, kind.ctx_name()) {
            if index != 0 {
                return Err(syn::Error::new(
                    param.ty.span(),
                    format!("`{}` must be the first parameter", kind.ctx_name()),
                ));
            }
            ctx = Some(state);
            continue;
        }
        if is_ctx(&param.ty, "Ctx") {
            return Err(syn::Error::new(
                param.ty.span(),
                format!(
                    "`Ctx` is for commands; a {} handler takes `{}`",
                    kind.attribute(),
                    kind.ctx_name()
                ),
            ));
        }
        let Pat::Ident(pat) = &*param.pat else {
            return Err(syn::Error::new(
                param.pat.span(),
                "parameters must be plain names",
            ));
        };
        if matches!(*param.ty, Type::Reference(_)) {
            return Err(syn::Error::new(
                param.ty.span(),
                "custom_id state is decoded into owned values: use `String`, not `&str`",
            ));
        }
        let arg = (pat.ident.clone(), (*param.ty).clone());
        if kind == Kind::Modal && form.is_none() {
            form = Some(arg);
        } else {
            state_args.push(arg);
        }
    }
    let form = match (kind, form) {
        (Kind::Modal, None) => {
            return Err(syn::Error::new(
                function.sig.span(),
                "modal handlers take the form (a `#[derive(Modal)]` type) after `ctx`",
            ));
        }
        (_, form) => form,
    };

    let has_ctx = ctx.is_some();
    let (state, impl_generics, where_clause) =
        resolve_state(&mut function, ctx, state_param, kind.ctx_name())?;

    let vis = &function.vis;
    let names: Vec<_> = state_args.iter().map(|(name, _)| name).collect();
    let types: Vec<_> = state_args.iter().map(|(_, ty)| ty).collect();
    let ctx_arg = has_ctx.then(|| quote!(::composure::command::Ctx::new(interaction, state),));
    let form_arg = form.as_ref().map(|(name, _)| quote!(#name,));
    let call = if function.sig.asyncness.is_some() {
        quote! { #ident(#ctx_arg #form_arg #(#names),*).await }
    } else {
        quote! { #ident(#ctx_arg #form_arg #(#names),*) }
    };

    let (definition, handler, interaction) = match kind {
        Kind::Component => (
            quote!(ComponentDefinition),
            quote!(ComponentHandler),
            quote!(MessageComponentInteraction),
        ),
        Kind::Modal => (
            quote!(ModalDefinition),
            quote!(ModalHandler),
            quote!(ModalSubmitInteraction),
        ),
    };
    let extract_form = form.as_ref().map(|(name, ty)| {
        quote! {
            let #name = <#ty as ::composure::command::Modal>::extract(&interaction.data)?;
        }
    });
    let show = form.as_ref().map(|(_, ty)| {
        quote! {
            /// The form as a modal response whose submit comes back to this handler with the
            /// given state
            #[allow(dead_code)]
            #vis fn show(
                #(#names: #types),*
            ) -> ::core::result::Result<
                ::composure::types::InteractionResponse,
                ::composure::command::custom_id::CustomIdError,
            > {
                ::core::result::Result::Ok(<#ty as ::composure::command::Modal>::response(
                    Self::custom_id(#(#names),*)?,
                ))
            }
        }
    });
    let docs: Vec<_> = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .collect();

    // A braced struct lives only in the type namespace, so the function keeps its name
    Ok(quote! {
        #function

        #(#docs)*
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, Default)]
        #vis struct #ident {}

        impl #ident {
            /// A `custom_id` routed to this handler, carrying the given state
            #[allow(dead_code)]
            #vis fn custom_id(
                #(#names: #types),*
            ) -> ::core::result::Result<
                ::std::string::String,
                ::composure::command::custom_id::CustomIdError,
            > {
                ::composure::command::custom_id::encode(#prefix, &(#(&#names,)*))
            }

            #show
        }

        impl ::composure::command::#definition for #ident {
            const PREFIX: &'static str = #prefix;
        }

        impl #impl_generics ::composure::command::#handler<#state> for #ident #where_clause {
            async fn invoke(
                interaction: &::composure::types::#interaction,
                state: &#state,
                fields: ::core::option::Option<&str>,
            ) -> ::core::result::Result<
                ::composure::types::InteractionResponse,
                ::composure::command::HandlerError,
            > {
                let _ = state;
                let (#(#names,)*): (#(#types,)*) =
                    ::composure::command::decode_state(#prefix, fields)?;
                #extract_form
                ::composure::command::CommandOutput::into_result(#call)
            }
        }
    })
}

/// The State a handler runs with, given its context parameter (`ctx`: `None` without one,
/// `Some(None)` without a State argument): a concrete type or the function's type parameter.
/// Adds `__S` for `Ctx` / `Ctx<impl Bound>` and spells out the context's lifetime. Returns the
/// State and the generics and where clause for the handler trait impl.
pub(crate) fn resolve_state(
    function: &mut ItemFn,
    ctx: Option<Option<Type>>,
    state_param: Option<Ident>,
    ctx_name: &str,
) -> syn::Result<(Type, TokenStream, TokenStream)> {
    // The State: a concrete type or the function's type parameter; `Ctx` and
    // `Ctx<impl Bound>` add one named `__S`
    let has_ctx = ctx.is_some();
    let state: Type = match ctx {
        Some(None) => {
            if let Some(param) = &state_param {
                return Err(syn::Error::new(
                    param.span(),
                    format!(
                        "use the type parameter as the State: `ctx: {}<S>`",
                        ctx_name
                    ),
                ));
            }
            function.sig.generics.params.push(parse_quote!(__S));
            parse_quote!(__S)
        }
        Some(Some(Type::ImplTrait(bounds))) => {
            let bounds = bounds.bounds;
            function
                .sig
                .generics
                .params
                .push(parse_quote!(__S: #bounds));
            parse_quote!(__S)
        }
        Some(Some(ty)) => ty,
        None => {
            if let Some(param) = &state_param {
                return Err(syn::Error::new(
                    param.span(),
                    format!(
                        "handlers can only be generic over their State: add `ctx: {}<{param}>`",
                        ctx_name
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
        // Spell out the lifetime: async fns reject an elided one
        segment.arguments = PathArguments::AngleBracketed(parse_quote!(<'_, #state>));
    }
    let generic = has_ctx && function.sig.generics.type_params().next().is_some();
    let (impl_generics, where_clause) = if generic {
        let (impl_generics, _, where_clause) = function.sig.generics.split_for_impl();
        (quote!(#impl_generics), quote!(#where_clause))
    } else if has_ctx {
        (quote!(), quote!())
    } else {
        (quote!(<__S>), quote!())
    };
    Ok((state, impl_generics, where_clause))
}

fn validate_prefix(prefix: &LitStr) -> syn::Result<()> {
    let value = prefix.value();
    if value.is_empty() || value.chars().count() > MAX_PREFIX_LEN {
        return Err(syn::Error::new(
            prefix.span(),
            "the prefix must be 1 to 100 characters",
        ));
    }
    if value.contains([':', '\\']) {
        return Err(syn::Error::new(
            prefix.span(),
            "the prefix cannot contain `:` or `\\`",
        ));
    }
    Ok(())
}

/// The function's one type parameter, which must be its State
pub(crate) fn state_param(function: &ItemFn) -> syn::Result<Option<Ident>> {
    let mut param = None;
    for generic in &function.sig.generics.params {
        match generic {
            GenericParam::Type(ty) if param.is_none() => param = Some(ty.ident.clone()),
            _ => {
                return Err(syn::Error::new(
                    generic.span(),
                    "handlers can only be generic over their State",
                ));
            }
        }
    }
    Ok(param)
}

pub(crate) fn is_ctx(ty: &Type, name: &str) -> bool {
    matches!(ty, Type::Path(path) if path.path.segments.last().is_some_and(|s| s.ident == name))
}

/// `Some(state)` if `ty` is the `name` context: `Some(None)` without a State argument
pub(crate) fn ctx_state(ty: &Type, name: &str) -> Option<Option<Type>> {
    if !is_ctx(ty, name) {
        return None;
    }
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return Some(None);
    };
    Some(args.args.iter().find_map(|arg| match arg {
        GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    }))
}
