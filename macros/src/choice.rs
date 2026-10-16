//! `#[derive(Choice)]`: a fieldless enum as an Option restricted to its variants.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitInt, LitStr, spanned::Spanned};

enum Value {
    Default,
    String(String),
    Integer(i64),
}

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let ident = &input.ident;
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(
            ident.span(),
            "`Choice` can only be derived for enums",
        ));
    };
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new(
            ident.span(),
            "`Choice` enums cannot be generic",
        ));
    }
    if data.variants.is_empty() || data.variants.len() > 25 {
        return Err(syn::Error::new(
            ident.span(),
            "`Choice` enums need 1-25 variants",
        ));
    }

    let mut variants = Vec::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new(
                variant.span(),
                "`Choice` variants cannot have fields",
            ));
        }
        let mut name = variant.ident.to_string();
        let mut value = Value::Default;
        for attr in variant.attrs.iter().filter(|a| a.path().is_ident("choice")) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let lit: LitStr = meta.value()?.parse()?;
                    name = lit.value();
                    let len = name.chars().count();
                    if !(1..=100).contains(&len) {
                        return Err(syn::Error::new(
                            lit.span(),
                            "choice names must be 1-100 characters",
                        ));
                    }
                } else if meta.path.is_ident("value") {
                    let input = meta.value()?;
                    value = if input.peek(LitStr) {
                        let lit: LitStr = input.parse()?;
                        if lit.value().chars().count() > 100 {
                            return Err(syn::Error::new(
                                lit.span(),
                                "choice values must be at most 100 characters",
                            ));
                        }
                        Value::String(lit.value())
                    } else {
                        let expr: syn::Expr = input.parse()?;
                        Value::Integer(crate::meta::number(&expr)? as i64)
                    };
                } else {
                    return Err(meta.error("unknown choice setting; expected `name` or `value`"));
                }
                Ok(())
            })?;
        }
        variants.push((variant.ident.clone(), name, value));
    }

    let integer = variants
        .iter()
        .any(|(_, _, v)| matches!(v, Value::Integer(_)));
    if integer
        && let Some((ident, _, _)) = variants
            .iter()
            .find(|(_, _, v)| !matches!(v, Value::Integer(_)))
    {
        return Err(syn::Error::new(
            ident.span(),
            "integer choices need `#[choice(value = N)]` on every variant",
        ));
    }

    let (option_type, definition, extract) = if integer {
        let pairs = variants.iter().map(|(_, name, value)| {
            let Value::Integer(value) = value else {
                unreachable!()
            };
            quote!((#name, #value))
        });
        let arms = variants.iter().map(|(variant, _, value)| {
            let Value::Integer(value) = value else {
                unreachable!()
            };
            let value = LitInt::new(&value.to_string(), variant.span());
            quote!(#value => ::core::result::Result::Ok(Self::#variant),)
        });
        (
            4u8,
            quote! { ::composure::command::integer_choices(meta, required, &[#(#pairs),*]) },
            quote! {
                match ::composure::command::integer_value(option)? {
                    #(#arms)*
                    _ => ::core::result::Result::Err(::composure::command::OptionError::UnknownChoice),
                }
            },
        )
    } else {
        let values: Vec<String> = variants
            .iter()
            .map(|(variant, _, value)| match value {
                Value::String(value) => value.clone(),
                _ => variant.to_string(),
            })
            .collect();
        let pairs = variants
            .iter()
            .zip(&values)
            .map(|((_, name, _), value)| quote!((#name, #value)));
        let arms = variants
            .iter()
            .zip(&values)
            .map(|((variant, _, _), value)| quote!(#value => ::core::result::Result::Ok(Self::#variant),));
        (
            3u8,
            quote! { ::composure::command::string_choices(meta, required, &[#(#pairs),*]) },
            quote! {
                match ::composure::command::string_value(option)? {
                    #(#arms)*
                    _ => ::core::result::Result::Err(::composure::command::OptionError::UnknownChoice),
                }
            },
        )
    };

    Ok(quote! {
        impl ::composure::command::OptionType for #ident {
            const OPTION_TYPE: u8 = #option_type;

            fn definition(
                meta: &::composure::command::OptionMeta,
                required: bool,
            ) -> ::composure::definition::ApplicationCommandOption {
                #definition
            }

            fn extract(
                option: ::core::option::Option<&::composure::models::ApplicationCommandInteractionDataOption>,
                _data: &::composure::models::ApplicationCommandInteractionData,
            ) -> ::core::result::Result<Self, ::composure::command::OptionError> {
                #extract
            }
        }
    })
}
