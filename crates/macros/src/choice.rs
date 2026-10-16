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
                        Value::Integer(crate::meta::integer(&expr)?)
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

    for (i, (variant, name, _)) in variants.iter().enumerate() {
        if variants[..i].iter().any(|(_, other, _)| other == name) {
            return Err(syn::Error::new(
                variant.span(),
                format!("duplicate choice name `{name}`"),
            ));
        }
    }

    let names = variants
        .iter()
        .map(|(variant, name, _)| quote!(Self::#variant => #name,));
    let (option_type, definition, extract, value_type, value_arms) = if integer {
        let pairs = variants.iter().map(|(_, name, value)| {
            let Value::Integer(value) = value else {
                unreachable!()
            };
            quote!((#name, #value))
        });
        let values: Vec<i64> = variants
            .iter()
            .map(|(_, _, value)| match value {
                Value::Integer(value) => *value,
                _ => unreachable!(),
            })
            .collect();
        duplicate_value(&variants, &values)?;
        let arms = variants
            .iter()
            .zip(&values)
            .map(|((variant, _, _), value)| {
                let value = LitInt::new(&value.to_string(), variant.span());
                quote!(#value => ::core::result::Result::Ok(Self::#variant),)
            });
        let value_arms: Vec<_> = variants
            .iter()
            .zip(&values)
            .map(|((variant, _, _), value)| quote!(Self::#variant => #value,))
            .collect();
        (
            4u8,
            quote! { ::composure::command::integer_choices(meta, required, &[#(#pairs),*]) },
            quote! {
                match ::composure::command::integer_value(option)? {
                    #(#arms)*
                    _ => ::core::result::Result::Err(::composure::command::OptionError::UnknownChoice),
                }
            },
            quote!(i64),
            value_arms,
        )
    } else {
        let values: Vec<String> = variants
            .iter()
            .map(|(variant, _, value)| match value {
                Value::String(value) => value.clone(),
                _ => variant.to_string(),
            })
            .collect();
        duplicate_value(&variants, &values)?;
        let value_arms: Vec<_> = variants
            .iter()
            .zip(&values)
            .map(|((variant, _, _), value)| quote!(Self::#variant => #value,))
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
            quote!(&'static str),
            value_arms,
        )
    };

    Ok(quote! {
        impl #ident {
            /// The name users see for this choice
            pub const fn name(&self) -> &'static str {
                match self {
                    #(#names)*
                }
            }

            /// The value Discord sends for this choice
            pub const fn value(&self) -> #value_type {
                match self {
                    #(#value_arms)*
                }
            }
        }

        impl ::composure::command::OptionType<'_> for #ident {
            const OPTION_TYPE: u8 = #option_type;

            fn definition(
                meta: &::composure::command::OptionMeta,
                required: bool,
            ) -> ::composure::definition::ApplicationCommandOption {
                #definition
            }

            fn extract(
                option: ::core::option::Option<&::composure::types::ApplicationCommandInteractionDataOption>,
                _data: &::composure::types::ApplicationCommandInteractionData,
            ) -> ::core::result::Result<Self, ::composure::command::OptionError> {
                #extract
            }
        }
    })
}

fn duplicate_value<T: PartialEq + std::fmt::Display>(
    variants: &[(syn::Ident, String, Value)],
    values: &[T],
) -> syn::Result<()> {
    for (i, value) in values.iter().enumerate() {
        if values[..i].contains(value) {
            return Err(syn::Error::new(
                variants[i].0.span(),
                format!("duplicate choice value `{value}`"),
            ));
        }
    }
    Ok(())
}
