//! `#[autocomplete]`: keeps the function and adds a same-named empty struct implementing
//! `Autocomplete`, for `#[option("...", autocomplete = name)]` to refer to.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, Pat, ReturnType, Type, spanned::Spanned};

use crate::handler::{ctx_state, resolve_state, state_param};

pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !args.is_empty() {
        return Err(syn::Error::new(
            args.span(),
            "`#[autocomplete]` takes no settings",
        ));
    }
    let mut function: ItemFn = syn::parse2(item)?;
    let ident = function.sig.ident.clone();
    if matches!(function.sig.output, ReturnType::Default) {
        return Err(syn::Error::new(
            function.sig.span(),
            "autocomplete handlers return their suggestions, e.g. `-> Vec<String>`",
        ));
    }
    let state_param = state_param(&function)?;

    let mut ctx = None;
    let mut inputs = 0;
    for (index, input) in function.sig.inputs.iter().enumerate() {
        let FnArg::Typed(param) = input else {
            return Err(syn::Error::new(
                input.span(),
                "autocomplete handlers cannot take `self`",
            ));
        };
        if let Some(state) = ctx_state(&param.ty, "Ctx") {
            if index != 0 {
                return Err(syn::Error::new(
                    param.ty.span(),
                    "`ctx` must be the first parameter",
                ));
            }
            ctx = Some(state);
            continue;
        }
        let is_str = matches!(&*param.ty, Type::Reference(r) if matches!(&*r.elem, Type::Path(p) if p.path.is_ident("str")));
        if !is_str || !matches!(&*param.pat, Pat::Ident(_)) {
            return Err(syn::Error::new(
                param.span(),
                "autocomplete handlers take what the user typed as `input: &str`",
            ));
        }
        inputs += 1;
    }
    if inputs != 1 {
        return Err(syn::Error::new(
            function.sig.span(),
            "autocomplete handlers take an optional `ctx: Ctx` and then `input: &str`",
        ));
    }

    let has_ctx = ctx.is_some();
    let (state, impl_generics, where_clause) =
        resolve_state(&mut function, ctx, state_param, "Ctx")?;
    let ctx_arg = has_ctx.then(|| quote!(ctx,));
    let call = if function.sig.asyncness.is_some() {
        quote! { #ident(#ctx_arg input).await }
    } else {
        quote! { #ident(#ctx_arg input) }
    };
    let vis = &function.vis;
    let docs: Vec<_> = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .collect();

    Ok(quote! {
        #function

        #(#docs)*
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, Default)]
        #vis struct #ident {}

        impl #impl_generics ::composure::command::Autocomplete<#state> for #ident #where_clause {
            async fn complete(
                ctx: ::composure::command::Ctx<'_, #state>,
                input: &str,
            ) -> ::core::result::Result<
                ::composure::types::InteractionResponse,
                ::composure::command::HandlerError,
            > {
                let _ = ctx;
                ::composure::command::AutocompleteOutput::into_result(#call)
            }
        }
    })
}
