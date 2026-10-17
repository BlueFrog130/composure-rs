//! `#[derive(Modal)]`: a struct whose fields are modal inputs, each wrapped in a Label. The field
//! attribute picks the input; the field type decides required-ness and how the value is read.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    Data, DeriveInput, Fields, Ident, LitBool, LitInt, LitStr, Type, meta::ParseNestedMeta,
    spanned::Spanned,
};

const MAX_TOP_LEVEL: usize = 5;
const MAX_TITLE: usize = 45;
const MAX_LABEL: usize = 45;
const MAX_DESCRIPTION: usize = 100;
const MAX_INPUT_LEN: i32 = 4000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Input,
    Select,
    UserSelect,
    RoleSelect,
    MentionableSelect,
    ChannelSelect,
    Radio,
    Checkboxes,
    Checkbox,
    FileUpload,
}

impl Kind {
    const ALL: [(&'static str, Kind); 10] = [
        ("input", Kind::Input),
        ("select", Kind::Select),
        ("user_select", Kind::UserSelect),
        ("role_select", Kind::RoleSelect),
        ("mentionable_select", Kind::MentionableSelect),
        ("channel_select", Kind::ChannelSelect),
        ("radio", Kind::Radio),
        ("checkboxes", Kind::Checkboxes),
        ("checkbox", Kind::Checkbox),
        ("file_upload", Kind::FileUpload),
    ];

    fn name(self) -> &'static str {
        Kind::ALL.iter().find(|(_, k)| *k == self).unwrap().0
    }

    /// The `shape` flag the field type must read, and what that means in words
    fn shape(self) -> (TokenStream, &'static str) {
        match self {
            Kind::Input | Kind::Radio => (
                quote!(::composure::command::shape::TEXT),
                "a single value (String, Option<String>, or a Choice enum)",
            ),
            Kind::Checkbox => (quote!(::composure::command::shape::CHECKED), "a bool"),
            _ => (
                quote!(::composure::command::shape::VALUES),
                "selections (a value, Option, or Vec)",
            ),
        }
    }

    fn has_options(self) -> bool {
        matches!(self, Kind::Select | Kind::Radio | Kind::Checkboxes)
    }

    /// Most selections Discord allows
    fn max_values(self) -> Option<u8> {
        match self {
            Kind::Select
            | Kind::UserSelect
            | Kind::RoleSelect
            | Kind::MentionableSelect
            | Kind::ChannelSelect => Some(25),
            Kind::Checkboxes | Kind::FileUpload => Some(10),
            _ => None,
        }
    }
}

#[derive(Default)]
struct Settings {
    custom_id: Option<String>,
    label: Option<String>,
    description: Option<String>,
    placeholder: Option<String>,
    min: Option<u8>,
    max: Option<u8>,
    options: Vec<(String, String)>,
    // input
    paragraph: bool,
    min_length: Option<i32>,
    max_length: Option<i32>,
    value: Option<String>,
    // channel select
    channel_types: Option<Vec<Ident>>,
    // checkbox
    default: bool,
    // file upload
    file_types: Option<Vec<String>>,
}

struct Field {
    ident: Ident,
    ty: Type,
    kind: Kind,
    settings: Settings,
}

pub fn expand(item: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(item)?;
    let ident = &input.ident;
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields,
            _ => return Err(not_named(&input)),
        },
        _ => return Err(not_named(&input)),
    };

    let mut title: Option<LitStr> = None;
    let mut texts: Vec<LitStr> = Vec::new();
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("modal")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("title") {
                title = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("text") {
                texts.push(meta.value()?.parse()?);
            } else {
                return Err(meta.error("unknown modal setting; expected `title` or `text`"));
            }
            Ok(())
        })?;
    }
    let title = title.ok_or_else(|| {
        syn::Error::new(
            ident.span(),
            "missing title: add `#[modal(title = \"...\")]`",
        )
    })?;
    check_len(&title, 1, MAX_TITLE, "the title")?;

    let count = fields.named.len() + texts.len();
    if fields.named.is_empty() || count > MAX_TOP_LEVEL {
        return Err(syn::Error::new(
            fields.span(),
            "a modal has 1 to 5 fields and `text`s in total",
        ));
    }

    let mut parsed: Vec<Field> = Vec::new();
    for field in &fields.named {
        let ident = field.ident.clone().expect("named field");
        let mut kind = None;
        let mut settings = Settings::default();
        for attr in &field.attrs {
            let Some((_, k)) = Kind::ALL
                .iter()
                .find(|(name, _)| attr.path().is_ident(name))
            else {
                continue;
            };
            if kind.replace(*k).is_some() {
                return Err(syn::Error::new(
                    attr.span(),
                    "a field is one input: use one of input, select, user_select, role_select, mentionable_select, channel_select, radio, checkboxes, checkbox, file_upload",
                ));
            }
            if !matches!(attr.meta, syn::Meta::Path(_)) {
                attr.parse_nested_meta(|meta| parse_setting(&meta, *k, &mut settings))?;
            }
        }
        let kind = kind.unwrap_or(Kind::Input);
        validate(&ident, kind, &settings)?;
        parsed.push(Field {
            ident,
            ty: field.ty.clone(),
            kind,
            settings,
        });
    }

    for (i, field) in parsed.iter().enumerate() {
        let id = custom_id(field);
        if parsed[..i].iter().any(|other| custom_id(other) == id) {
            return Err(syn::Error::new(
                field.ident.span(),
                format!("duplicate input id `{id}`"),
            ));
        }
    }

    let metas = parsed.iter().map(field_meta);
    let n = parsed.len();
    let components = parsed
        .iter()
        .enumerate()
        .map(|(i, field)| component(i, field));
    let extracts = parsed.iter().enumerate().map(|(i, field)| {
        let (ident, ty) = (&field.ident, &field.ty);
        quote! { #ident: Self::__FIELDS[#i].extract::<#ty>(data)? }
    });
    let checks = parsed.iter().map(check);

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            #[doc(hidden)]
            const __FIELDS: [::composure::command::FieldMeta; #n] = [#(#metas),*];
        }

        // One block per field, so every mismatch is reported
        #(const _: () = { #checks };)*

        impl #impl_generics ::composure::command::Modal for #ident #ty_generics #where_clause {
            const TITLE: &'static str = #title;

            fn components() -> ::std::vec::Vec<::composure::types::Component> {
                ::std::vec![
                    #(::composure::types::Component::text(#texts),)*
                    #(#components),*
                ]
            }

            fn extract(
                data: &::composure::types::ModalSubmitData,
            ) -> ::core::result::Result<Self, ::composure::command::HandlerError> {
                ::core::result::Result::Ok(Self { #(#extracts),* })
            }
        }
    })
}

fn not_named(input: &DeriveInput) -> syn::Error {
    syn::Error::new(
        input.span(),
        "`#[derive(Modal)]` needs a struct with named fields",
    )
}

fn parse_setting(meta: &ParseNestedMeta, kind: Kind, settings: &mut Settings) -> syn::Result<()> {
    let path = &meta.path;
    let allowed = |names: &[&str]| names.iter().any(|name| path.is_ident(name));
    let common = ["label", "description", "id"];
    let specific: &[&str] = match kind {
        Kind::Input => &["paragraph", "min_len", "max_len", "placeholder", "value"],
        Kind::Select => &["placeholder", "min", "max", "option"],
        Kind::UserSelect | Kind::RoleSelect | Kind::MentionableSelect => {
            &["placeholder", "min", "max"]
        }
        Kind::ChannelSelect => &["placeholder", "min", "max", "channel_types"],
        Kind::Radio => &["option"],
        Kind::Checkboxes => &["min", "max", "option"],
        Kind::Checkbox => &["default"],
        Kind::FileUpload => &["min", "max", "file_types"],
    };
    if !allowed(&common) && !allowed(specific) {
        let expected: Vec<_> = common.iter().chain(specific).copied().collect();
        return Err(meta.error(format!(
            "unknown `{}` setting; expected one of {}",
            kind.name(),
            expected.join(", ")
        )));
    }

    if path.is_ident("label") {
        let label: LitStr = meta.value()?.parse()?;
        check_len(&label, 1, MAX_LABEL, "the label")?;
        settings.label = Some(label.value());
    } else if path.is_ident("description") {
        let description: LitStr = meta.value()?.parse()?;
        check_len(&description, 1, MAX_DESCRIPTION, "the description")?;
        settings.description = Some(description.value());
    } else if path.is_ident("id") {
        let id: LitStr = meta.value()?.parse()?;
        check_len(&id, 1, 100, "the id")?;
        settings.custom_id = Some(id.value());
    } else if path.is_ident("placeholder") {
        let placeholder: LitStr = meta.value()?.parse()?;
        let max = if kind == Kind::Input { 100 } else { 150 };
        check_len(&placeholder, 1, max, "the placeholder")?;
        settings.placeholder = Some(placeholder.value());
    } else if path.is_ident("min") {
        settings.min = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
    } else if path.is_ident("max") {
        let max: LitInt = meta.value()?.parse()?;
        let value: u8 = max.base10_parse()?;
        if value == 0 {
            return Err(syn::Error::new(max.span(), "`max` must be at least 1"));
        }
        settings.max = Some(value);
    } else if path.is_ident("option") {
        let content;
        syn::parenthesized!(content in meta.input);
        let label: LitStr = content.parse()?;
        content.parse::<syn::Token![,]>()?;
        let value: LitStr = content.parse()?;
        check_len(&label, 1, 100, "an option label")?;
        check_len(&value, 1, 100, "an option value")?;
        settings.options.push((label.value(), value.value()));
    } else if path.is_ident("paragraph") {
        settings.paragraph = true;
    } else if path.is_ident("min_len") {
        settings.min_length = Some(length(&meta.value()?.parse()?, 0)?);
    } else if path.is_ident("max_len") {
        settings.max_length = Some(length(&meta.value()?.parse()?, 1)?);
    } else if path.is_ident("value") {
        let value: LitStr = meta.value()?.parse()?;
        check_len(&value, 0, MAX_INPUT_LEN as usize, "the value")?;
        settings.value = Some(value.value());
    } else if path.is_ident("channel_types") {
        settings.channel_types = Some(crate::meta::ident_list(meta)?);
    } else if path.is_ident("default") {
        settings.default = if meta.input.peek(syn::Token![=]) {
            meta.value()?.parse::<LitBool>()?.value
        } else {
            true
        };
    } else if path.is_ident("file_types") {
        let content;
        syn::parenthesized!(content in meta.input);
        let types =
            content.parse_terminated(<LitStr as syn::parse::Parse>::parse, syn::Token![,])?;
        if types.is_empty() || types.len() > 10 {
            return Err(meta.error("`file_types` lists 1 to 10 types"));
        }
        settings.file_types = Some(types.iter().map(LitStr::value).collect());
    }
    Ok(())
}

fn validate(ident: &Ident, kind: Kind, settings: &Settings) -> syn::Result<()> {
    let span = ident.span();
    let error = |message: String| Err(syn::Error::new(span, message));
    if let (Some(min), Some(max)) = (settings.min, settings.max)
        && min > max
    {
        return error("`min` is greater than `max`".into());
    }
    if let (Some(min), Some(max)) = (settings.min_length, settings.max_length)
        && min > max
    {
        return error("`min_len` is greater than `max_len`".into());
    }
    if let Some(limit) = kind.max_values()
        && settings
            .min
            .into_iter()
            .chain(settings.max)
            .any(|v| v > limit)
    {
        return error(format!(
            "`#[{}]` allows at most {limit} selections",
            kind.name()
        ));
    }
    let options = settings.options.len();
    match kind {
        Kind::Select if options > 25 => error("a select has at most 25 options".into()),
        Kind::Radio if options == 1 || options > 10 => {
            error("a radio group has 2 to 10 options".into())
        }
        Kind::Checkboxes if options > 10 => error("a checkbox group has 1 to 10 options".into()),
        _ => Ok(()),
    }
}

fn custom_id(field: &Field) -> String {
    field
        .settings
        .custom_id
        .clone()
        .unwrap_or_else(|| field.ident.to_string())
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

fn option_str(value: &Option<String>) -> TokenStream {
    match value {
        Some(v) => quote!(::core::option::Option::Some(#v)),
        None => quote!(::core::option::Option::None),
    }
}

fn option_lit<T: quote::ToTokens>(value: Option<T>) -> TokenStream {
    match value {
        Some(v) => quote!(::core::option::Option::Some(#v)),
        None => quote!(::core::option::Option::None),
    }
}

fn field_meta(field: &Field) -> TokenStream {
    let Settings {
        label,
        description,
        placeholder,
        min,
        max,
        ..
    } = &field.settings;
    let ty = &field.ty;
    let custom_id = custom_id(field);
    let label = label
        .clone()
        .unwrap_or_else(|| default_label(&field.ident.to_string()));
    let description = option_str(description);
    let placeholder = option_str(placeholder);
    let (min, max) = (option_lit(*min), option_lit(*max));
    quote! {
        ::composure::command::FieldMeta {
            custom_id: #custom_id,
            label: #label,
            description: #description,
            placeholder: #placeholder,
            min: #min,
            max: #max,
            required: <#ty as ::composure::command::ModalValue>::REQUIRED,
            many: <#ty as ::composure::command::ModalValue>::MANY,
        }
    }
}

fn options(field: &Field) -> TokenStream {
    let ty = &field.ty;
    if field.settings.options.is_empty() {
        quote!(<#ty as ::composure::command::ModalOptions>::OPTIONS)
    } else {
        let pairs = field
            .settings
            .options
            .iter()
            .map(|(label, value)| quote!((#label, #value)));
        quote!(&[#(#pairs),*])
    }
}

fn component(i: usize, field: &Field) -> TokenStream {
    let meta = quote!(Self::__FIELDS[#i]);
    let settings = &field.settings;
    match field.kind {
        Kind::Input => {
            let paragraph = settings.paragraph;
            let (min, max) = (
                option_lit(settings.min_length),
                option_lit(settings.max_length),
            );
            let value = option_str(&settings.value);
            quote! {
                #meta.text_input(&::composure::command::TextInputMeta {
                    paragraph: #paragraph,
                    min_length: #min,
                    max_length: #max,
                    value: #value,
                })
            }
        }
        Kind::Select => {
            let options = options(field);
            quote!(#meta.string_select(#options))
        }
        Kind::UserSelect => quote!(#meta.user_select()),
        Kind::RoleSelect => quote!(#meta.role_select()),
        Kind::MentionableSelect => quote!(#meta.mentionable_select()),
        Kind::ChannelSelect => {
            let types = match &settings.channel_types {
                Some(types) => {
                    quote!(::core::option::Option::Some(&[#(::composure::types::ChannelType::#types),*]))
                }
                None => quote!(::core::option::Option::None),
            };
            quote!(#meta.channel_select(#types))
        }
        Kind::Radio => {
            let options = options(field);
            quote!(#meta.radio(#options))
        }
        Kind::Checkboxes => {
            let options = options(field);
            quote!(#meta.checkboxes(#options))
        }
        Kind::Checkbox => {
            let default = settings.default;
            quote!(#meta.checkbox(#default))
        }
        Kind::FileUpload => {
            let types = match &settings.file_types {
                Some(types) => quote!(::core::option::Option::Some(&[#(#types),*])),
                None => quote!(::core::option::Option::None),
            };
            quote!(#meta.file_upload(#types))
        }
    }
}

/// Compile-time checks needing the field type: it reads what the input submits, and Choice
/// option counts fit the input
fn check(field: &Field) -> TokenStream {
    let ty = &field.ty;
    let name = field.ident.to_string();
    let (shape, expected) = field.kind.shape();
    let message = format!(
        "field `{name}`: `#[{}]` submits {expected}",
        field.kind.name()
    );
    let mut checks = vec![quote_spanned! {ty.span()=>
        ::core::assert!(
            <#ty as ::composure::command::ModalValue>::SHAPE & #shape != 0,
            #message,
        );
    }];
    if field.kind.has_options() && field.settings.options.is_empty() {
        let (min, max, what) = match field.kind {
            Kind::Select => (1usize, 25usize, "a select has 1 to 25 options"),
            Kind::Radio => (2, 10, "a radio group has 2 to 10 options"),
            _ => (1, 10, "a checkbox group has 1 to 10 options"),
        };
        let message = format!("field `{name}`: {what}");
        checks.push(quote_spanned! {ty.span()=>
            ::core::assert!(
                {
                    let n = <#ty as ::composure::command::ModalOptions>::OPTIONS.len();
                    n >= #min && n <= #max
                },
                #message,
            );
        });
    }
    quote!(#(#checks)*)
}

fn check_len(lit: &LitStr, min: usize, max: usize, what: &str) -> syn::Result<()> {
    let len = lit.value().chars().count();
    if len < min || len > max {
        return Err(syn::Error::new(
            lit.span(),
            format!("{what} must be {min} to {max} characters"),
        ));
    }
    Ok(())
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
