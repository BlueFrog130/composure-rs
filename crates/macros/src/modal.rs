//! `#[derive(Modal)]`: a struct of text inputs shown as a modal and read back on submit.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitInt, LitStr, spanned::Spanned};

const MAX_FIELDS: usize = 5;
const MAX_TITLE: usize = 45;
const MAX_LABEL: usize = 45;
const MAX_INPUT_LEN: i32 = 4000;

struct Input {
    ident: syn::Ident,
    ty: syn::Type,
    custom_id: String,
    label: String,
    paragraph: bool,
    min_length: Option<i32>,
    max_length: Option<i32>,
    placeholder: Option<String>,
    value: Option<String>,
}

pub fn expand(item: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(item)?;
    let ident = &input.ident;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "`#[derive(Modal)]` needs a struct with named fields",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(
            input.span(),
            "`#[derive(Modal)]` needs a struct with named fields",
        ));
    };

    let mut title: Option<LitStr> = None;
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("modal")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("title") {
                title = Some(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(meta.error("unknown modal setting; expected `title`"))
            }
        })?;
    }
    let title = title.ok_or_else(|| {
        syn::Error::new(
            ident.span(),
            "missing title: add `#[modal(title = \"...\")]`",
        )
    })?;
    if title.value().is_empty() || title.value().chars().count() > MAX_TITLE {
        return Err(syn::Error::new(
            title.span(),
            "the title must be 1 to 45 characters",
        ));
    }

    if fields.named.is_empty() || fields.named.len() > MAX_FIELDS {
        return Err(syn::Error::new(fields.span(), "a modal has 1 to 5 fields"));
    }

    let mut inputs = Vec::new();
    for field in &fields.named {
        let ident = field.ident.clone().expect("named field");
        let mut input = Input {
            custom_id: ident.to_string(),
            label: default_label(&ident.to_string()),
            ident,
            ty: field.ty.clone(),
            paragraph: false,
            min_length: None,
            max_length: None,
            placeholder: None,
            value: None,
        };
        for attr in field.attrs.iter().filter(|a| a.path().is_ident("input")) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("label") {
                    let label: LitStr = meta.value()?.parse()?;
                    if label.value().is_empty() || label.value().chars().count() > MAX_LABEL {
                        return Err(syn::Error::new(
                            label.span(),
                            "the label must be 1 to 45 characters",
                        ));
                    }
                    input.label = label.value();
                } else if meta.path.is_ident("paragraph") {
                    input.paragraph = true;
                } else if meta.path.is_ident("min_len") {
                    input.min_length = Some(length(&meta.value()?.parse()?, 0)?);
                } else if meta.path.is_ident("max_len") {
                    input.max_length = Some(length(&meta.value()?.parse()?, 1)?);
                } else if meta.path.is_ident("placeholder") {
                    input.placeholder = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.path.is_ident("value") {
                    input.value = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.path.is_ident("id") {
                    input.custom_id = meta.value()?.parse::<LitStr>()?.value();
                } else {
                    return Err(meta.error(
                        "unknown input setting; expected label, paragraph, min_len, max_len, placeholder, value, id",
                    ));
                }
                Ok(())
            })?;
        }
        if let (Some(min), Some(max)) = (input.min_length, input.max_length)
            && min > max
        {
            return Err(syn::Error::new(
                input.ident.span(),
                "`min_len` is greater than `max_len`",
            ));
        }
        inputs.push(input);
    }

    let metas: Vec<_> = inputs.iter().map(meta).collect();
    let rows = inputs.iter().zip(&metas).map(|(input, meta)| {
        let ty = &input.ty;
        quote! { #meta.row(<#ty as ::composure::command::ModalField>::REQUIRED) }
    });
    let extracts = inputs.iter().zip(&metas).map(|(input, meta)| {
        let (ident, ty) = (&input.ident, &input.ty);
        quote! { #ident: #meta.extract::<#ty>(data)? }
    });

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics ::composure::command::Modal for #ident #ty_generics #where_clause {
            const TITLE: &'static str = #title;

            fn components() -> ::std::vec::Vec<::composure::types::ActionRow> {
                ::std::vec![#(#rows),*]
            }

            fn extract(
                data: &::composure::types::ModalSubmitData,
            ) -> ::core::result::Result<Self, ::composure::command::HandlerError> {
                ::core::result::Result::Ok(Self { #(#extracts),* })
            }
        }
    })
}

fn meta(input: &Input) -> TokenStream {
    let custom_id = &input.custom_id;
    let label = &input.label;
    let paragraph = input.paragraph;
    let option_i32 = |value: Option<i32>| match value {
        Some(v) => quote!(::core::option::Option::Some(#v)),
        None => quote!(::core::option::Option::None),
    };
    let option_str = |value: &Option<String>| match value {
        Some(v) => quote!(::core::option::Option::Some(#v)),
        None => quote!(::core::option::Option::None),
    };
    let (min, max) = (option_i32(input.min_length), option_i32(input.max_length));
    let (placeholder, value) = (option_str(&input.placeholder), option_str(&input.value));
    quote! {
        ::composure::command::InputMeta {
            custom_id: #custom_id,
            label: #label,
            paragraph: #paragraph,
            min_length: #min,
            max_length: #max,
            placeholder: #placeholder,
            value: #value,
        }
    }
}

/// `release_notes` -> `Release notes`
fn default_label(field: &str) -> String {
    let words = field.replace('_', " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => words,
    }
}

fn length(lit: &LitInt, min: i32) -> syn::Result<i32> {
    let value: i32 = lit.base10_parse()?;
    if !(min..=MAX_INPUT_LEN).contains(&value) {
        return Err(syn::Error::new(
            lit.span(),
            format!("length must be between {min} and 4000"),
        ));
    }
    Ok(value)
}
