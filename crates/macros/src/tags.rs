//! Every built-in `rsx!` tag and its attributes, with their docs: the one source for the hidden
//! `composure::command::tags` module (rust-analyzer completes tag and attribute names from it and
//! shows these docs on hover), for which attributes `rsx!` references there, and for the test
//! that the guide's tag reference lists them all.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

/// A built-in tag
pub struct TagDef {
    /// The tag name, as `Tag`'s variant is called
    pub name: &'static str,
    /// Other names for the same tag
    pub aliases: &'static [&'static str],
    /// The `composure::types` type it builds
    pub ty: &'static str,
    pub doc: &'static str,
    pub attrs: &'static [AttrDef],
    /// Fields of `ty` that are not attributes, so the attributes plus these are every field
    pub internal: &'static [&'static str],
}

/// An attribute of a tag
pub struct AttrDef {
    pub name: &'static str,
    /// Whether it is a field of the tag's type (else it is handled by the tag itself)
    pub field: bool,
    pub doc: &'static str,
}

const fn field(name: &'static str, doc: &'static str) -> AttrDef {
    AttrDef {
        name,
        field: true,
        doc,
    }
}

const fn special(name: &'static str, doc: &'static str) -> AttrDef {
    AttrDef {
        name,
        field: false,
        doc,
    }
}

const ID: AttrDef = field(
    "id",
    "`i32`: identifier unique within the message; Discord numbers components without one",
);

const ENTITY_SELECT_ATTRS: &[AttrDef] = &[
    ID,
    special(
        "on",
        "`on:select=handler(state..)` answers with a `#[component]` handler; `on:select:name=|ctx, values: T| ..` is an inline handler (`#[view]` only)",
    ),
    field(
        "custom_id",
        "`String`: answered by whatever handles this `custom_id`; max 100 characters",
    ),
    special(
        "name",
        "In a modal (required there): a string literal, the input's `custom_id`, matched to an `on:submit` parameter",
    ),
    field(
        "placeholder",
        "`String`: shown while nothing is selected; max 150 characters",
    ),
    special("min", "`i32`: fewest selections, 0-25 (default 1)"),
    field("min_values", "`i32`: fewest selections, 0-25 (default 1)"),
    special(
        "max",
        "`i32`: most selections, 1-25 (default 1; a `Vec` handler parameter makes it 25)",
    ),
    field(
        "max_values",
        "`i32`: most selections, 1-25 (default 1; a `Vec` handler parameter makes it 25)",
    ),
    field("disabled", "`bool`: not selectable; messages only"),
    field(
        "required",
        "`bool`: modals only; set from the submit parameter's type",
    ),
    field(
        "default_values",
        "`Vec<SelectDefaultValue>`: preselected entities, `vec![SelectDefaultValue::user(id)]`",
    ),
    field(
        "channel_types",
        "`Vec<ChannelType>`: `<ChannelSelect>`, the kinds of channel offered",
    ),
];

const SELECT_INTERNAL: &[&str] = &["t", "options"];

const NAME: AttrDef = special(
    "name",
    "*Required.* A string literal: the input's `custom_id`, matched to an `on:submit` parameter",
);

const MEDIA_URL: &str = "*Required.* An image URL, `attachment://<filename>`, a `File` to upload with the message (its `attachment://` URL is set for you), or an `&Attachment`";

pub const TAGS: &[TagDef] = &[
    TagDef {
        name: "Container",
        aliases: &[],
        ty: "Container",
        doc: "A frame around other components, like an embed, with an optional accent bar. Top level only.",
        attrs: &[
            ID,
            field(
                "accent_color",
                "`u32` (`0xRRGGBB`): color of the bar on the left; none when unset",
            ),
            field("spoiler", "`bool`: blurs the content until clicked"),
        ],
        internal: &["t", "components"],
    },
    TagDef {
        name: "Section",
        aliases: &[],
        ty: "Section",
        doc: "1-3 text displays beside an accessory: a `<Button>` or `<Thumbnail>` child.",
        attrs: &[
            ID,
            field(
                "accessory",
                "`impl Into<Component>`: the accessory as a value, instead of a child",
            ),
        ],
        internal: &["t", "components"],
    },
    TagDef {
        name: "Text",
        aliases: &["TextDisplay"],
        ty: "TextDisplay",
        doc: "Markdown text. Children: text and `{..}` only.",
        attrs: &[
            ID,
            field("content", "`String`: the text, instead of children"),
        ],
        internal: &["t"],
    },
    TagDef {
        name: "Thumbnail",
        aliases: &[],
        ty: "Thumbnail",
        doc: "A small image, as a `<Section>`'s accessory only.",
        attrs: &[
            ID,
            special("url", MEDIA_URL),
            special("src", MEDIA_URL),
            field("description", "`String`: alt text, max 1024 characters"),
            field("spoiler", "`bool`: blurs the image until clicked"),
        ],
        internal: &["t", "media"],
    },
    TagDef {
        name: "Gallery",
        aliases: &["MediaGallery"],
        ty: "MediaGallery",
        doc: "A grid of 1-10 images and videos: `<Item>`s, and `{..}` of `MediaGalleryItem`s.",
        attrs: &[ID],
        internal: &["t", "items"],
    },
    TagDef {
        name: "Item",
        aliases: &["MediaGalleryItem"],
        ty: "MediaGalleryItem",
        doc: "One image or video in a `<Gallery>`.",
        attrs: &[
            special("url", MEDIA_URL),
            special("src", MEDIA_URL),
            field("description", "`String`: alt text, max 1024 characters"),
            field("spoiler", "`bool`: blurs the media until clicked"),
        ],
        internal: &["media"],
    },
    TagDef {
        name: "File",
        aliases: &[],
        ty: "FileComponent",
        doc: "A file uploaded with the message, shown inline.",
        attrs: &[
            ID,
            special(
                "url",
                "*Required.* `attachment://<filename>`, or a `File` to upload with the message (its URL is set for you)",
            ),
            special(
                "src",
                "*Required.* `attachment://<filename>`, or a `File` to upload with the message (its URL is set for you)",
            ),
            field("spoiler", "`bool`: blurs the file until clicked"),
        ],
        internal: &["t", "file", "name", "size"],
    },
    TagDef {
        name: "Separator",
        aliases: &[],
        ty: "Separator",
        doc: "Vertical space, with a divider line by default.",
        attrs: &[
            ID,
            special("small", "Flag: small spacing (the default)"),
            special("large", "Flag: large spacing"),
            field("spacing", "`SeparatorSpacing`: the spacing, as a value"),
            field("divider", "`bool`: `false` for space only, no line"),
        ],
        internal: &["t"],
    },
    TagDef {
        name: "Row",
        aliases: &["ActionRow"],
        ty: "ActionRow",
        doc: "Up to 5 `<Button>`s, or one select.",
        attrs: &[ID],
        internal: &["t", "components"],
    },
    TagDef {
        name: "Button",
        aliases: &[],
        ty: "ButtonComponent",
        doc: "A button, in a `<Row>` or as a `<Section>`'s accessory. Children: the label.",
        attrs: &[
            ID,
            special("primary", "Flag: blurple"),
            special("secondary", "Flag: grey (the default)"),
            special("success", "Flag: green"),
            special("danger", "Flag: red"),
            field("style", "`ButtonStyle`: the style, as a value"),
            special(
                "on",
                "`on:click=handler(state..)` answers with a `#[component]` handler; `on:click:name=|ctx| ..` is an inline handler (`#[view]` only)",
            ),
            field(
                "custom_id",
                "`String`: answered by whatever handles this `custom_id`; max 100 characters",
            ),
            field(
                "url",
                "`String`: a link button (style `Link`), opening the URL; no `on:click` or `custom_id`",
            ),
            field(
                "sku_id",
                "`Snowflake`: a premium button (style `Premium`) offering the SKU",
            ),
            field(
                "label",
                "`String`: the label, instead of children; max 80 characters",
            ),
            field(
                "emoji",
                "`&str` or `PartialEmoji`: shown before the label, `\"👍\"` or a custom emoji",
            ),
            field("disabled", "`bool`: greyed out, not clickable"),
        ],
        internal: &["t"],
    },
    TagDef {
        name: "StringSelect",
        aliases: &["Select"],
        ty: "StringSelect",
        doc: "A dropdown of up to 25 text options: `<Option>`s, and `{..}` of `SelectOption`s.",
        attrs: &[
            ID,
            special(
                "on",
                "`on:select=handler(state..)` answers with a `#[component]` handler; `on:select:name=|ctx, values: T| ..` is an inline handler (`#[view]` only)",
            ),
            field(
                "custom_id",
                "`String`: answered by whatever handles this `custom_id`; max 100 characters",
            ),
            special(
                "name",
                "In a modal (required there): a string literal, the input's `custom_id`, matched to an `on:submit` parameter",
            ),
            special(
                "options",
                "A `#[derive(Choice)]` enum: one option per variant, before any `<Option>` children",
            ),
            field(
                "placeholder",
                "`String`: shown while nothing is selected; max 150 characters",
            ),
            special("min", "`i32`: fewest selections, 0-25 (default 1)"),
            field("min_values", "`i32`: fewest selections, 0-25 (default 1)"),
            special(
                "max",
                "`i32`: most selections, 1-25 (default 1; a `Vec` handler parameter makes it every option)",
            ),
            field(
                "max_values",
                "`i32`: most selections, 1-25 (default 1; a `Vec` handler parameter makes it every option)",
            ),
            field("disabled", "`bool`: not selectable; messages only"),
            field(
                "required",
                "`bool`: modals only; set from the submit parameter's type",
            ),
        ],
        internal: &["t", "options", "channel_types", "default_values"],
    },
    TagDef {
        name: "UserSelect",
        aliases: &[],
        ty: "UserSelect",
        doc: "Pick users, with Discord filling in the options.",
        attrs: ENTITY_SELECT_ATTRS,
        internal: SELECT_INTERNAL,
    },
    TagDef {
        name: "RoleSelect",
        aliases: &[],
        ty: "RoleSelect",
        doc: "Pick roles, with Discord filling in the options.",
        attrs: ENTITY_SELECT_ATTRS,
        internal: SELECT_INTERNAL,
    },
    TagDef {
        name: "MentionableSelect",
        aliases: &[],
        ty: "MentionableSelect",
        doc: "Pick users or roles, with Discord filling in the options.",
        attrs: ENTITY_SELECT_ATTRS,
        internal: SELECT_INTERNAL,
    },
    TagDef {
        name: "ChannelSelect",
        aliases: &[],
        ty: "ChannelSelect",
        doc: "Pick channels, with Discord filling in the options.",
        attrs: ENTITY_SELECT_ATTRS,
        internal: SELECT_INTERNAL,
    },
    TagDef {
        name: "Option",
        aliases: &[],
        ty: "SelectOption",
        doc: "An option of a select, `<RadioGroup>` or `<CheckboxGroup>`. Children: the label.",
        attrs: &[
            field(
                "value",
                "*Required.* `impl Into<String>`: what is submitted when chosen; max 100 characters",
            ),
            field(
                "label",
                "`String`: the label, instead of children; max 100 characters",
            ),
            field(
                "description",
                "`String`: a second line under the label; max 100 characters",
            ),
            field("emoji", "`&str` or `PartialEmoji`: selects only"),
            field("default", "`bool`: chosen initially"),
        ],
        internal: &[],
    },
    TagDef {
        name: "Modal",
        aliases: &[],
        ty: "ModalCallbackData",
        doc: "A modal, as a `ModalView`; the markup's only root. Children: text, `<Text>` and `<Label>`s, at most 5.",
        attrs: &[
            field(
                "title",
                "*Required.* `impl Into<String>`: shown at the top; max 45 characters",
            ),
            special(
                "on",
                "`on:submit=handler(state..)` submits to a `#[modal]` handler; `on:submit:name=|ctx, field: T, ..| ..` is an inline handler (`#[view]` only)",
            ),
            field(
                "custom_id",
                "`String`: submitted to whatever handles this `custom_id`",
            ),
        ],
        internal: &[
            "components",
            "tts",
            "content",
            "embeds",
            "allowed_mentions",
            "flags",
        ],
    },
    TagDef {
        name: "Label",
        aliases: &[],
        ty: "Label",
        doc: "A title and optional description above one modal input.",
        attrs: &[
            ID,
            special(
                "text",
                "*Required.* `impl Into<String>`: the title; max 45 characters",
            ),
            field(
                "label",
                "*Required.* `impl Into<String>`: the title; max 45 characters",
            ),
            field(
                "description",
                "`String`: under the title; max 100 characters",
            ),
        ],
        internal: &["t", "component"],
    },
    TagDef {
        name: "TextInput",
        aliases: &[],
        ty: "TextInput",
        doc: "Free text, in a `<Label>` in a modal.",
        attrs: &[
            ID,
            NAME,
            special("paragraph", "Flag: multi-line; single-line by default"),
            field("style", "`TextInputStyle`: the same, as a value"),
            special("min_len", "`i32`: shortest input, 0-4000"),
            field("min_length", "`i32`: shortest input, 0-4000"),
            special("max_len", "`i32`: longest input, 1-4000"),
            field("max_length", "`i32`: longest input, 1-4000"),
            field(
                "placeholder",
                "`String`: shown while empty; max 100 characters",
            ),
            field("value", "`String`: prefilled text; max 4000 characters"),
            field("required", "`bool`: set from the submit parameter's type"),
        ],
        internal: &["t", "custom_id", "label"],
    },
    TagDef {
        name: "RadioGroup",
        aliases: &[],
        ty: "RadioGroup",
        doc: "Pick one of 2-10 options, in a `<Label>` in a modal.",
        attrs: &[
            ID,
            NAME,
            field(
                "options",
                "A `#[derive(Choice)]` enum: one option per variant",
            ),
            field("required", "`bool`: set from the submit parameter's type"),
        ],
        internal: &["t", "custom_id"],
    },
    TagDef {
        name: "CheckboxGroup",
        aliases: &[],
        ty: "CheckboxGroup",
        doc: "Pick any of 1-10 options, in a `<Label>` in a modal.",
        attrs: &[
            ID,
            NAME,
            field(
                "options",
                "A `#[derive(Choice)]` enum: one option per variant",
            ),
            special("min", "`u8`: fewest picked, 0-10"),
            field("min_values", "`u8`: fewest picked, 0-10"),
            special("max", "`u8`: most picked, 1-10"),
            field("max_values", "`u8`: most picked, 1-10"),
            field("required", "`bool`: set from the submit parameter's type"),
        ],
        internal: &["t", "custom_id"],
    },
    TagDef {
        name: "Checkbox",
        aliases: &[],
        ty: "Checkbox",
        doc: "One yes/no, submitted as a `bool`, in a `<Label>` in a modal.",
        attrs: &[ID, NAME, field("default", "`bool`: checked initially")],
        internal: &["t", "custom_id"],
    },
    TagDef {
        name: "FileUpload",
        aliases: &[],
        ty: "FileUpload",
        doc: "Attach files, submitted as `Attachment`s, in a `<Label>` in a modal.",
        attrs: &[
            ID,
            NAME,
            special("min", "`u8`: fewest files, 0-10"),
            field("min_values", "`u8`: fewest files, 0-10"),
            special(
                "max",
                "`u8`: most files, 1-10 (a `Vec` parameter makes it 10)",
            ),
            field(
                "max_values",
                "`u8`: most files, 1-10 (a `Vec` parameter makes it 10)",
            ),
            field(
                "file_types",
                "`Vec<String>`: up to 10 of `\"image\"`, `\"video\"`, `\"audio\"`, or extensions like `\".pdf\"`",
            ),
            field("required", "`bool`: set from the submit parameter's type"),
        ],
        internal: &["t", "custom_id"],
    },
];

/// The tag named `name` (or one of its aliases)
pub fn find(name: &str) -> Option<&'static TagDef> {
    TAGS.iter()
        .find(|tag| tag.name == name || tag.aliases.contains(&name))
}

impl TagDef {
    pub fn has(&self, attr: &str) -> bool {
        self.attrs.iter().any(|a| a.name == attr)
    }

    /// Whether `partial` starts a special attribute's name. Field names complete through the
    /// field assignment the attribute expands to, without the second error a reference to a
    /// missing item adds.
    pub fn starts_special(&self, partial: &str) -> bool {
        self.attrs
            .iter()
            .any(|a| !a.field && a.name.starts_with(partial))
    }
}

/// Whether `partial` starts a tag's name (or an alias's)
pub fn starts(partial: &str) -> bool {
    TAGS.iter().any(|tag| {
        tag.name.starts_with(partial) || tag.aliases.iter().any(|a| a.starts_with(partial))
    })
}

/// `composure::command::tags`: one module per tag with one unit struct per attribute, all documented,
/// plus `ALL` for the guide's consistency test and an exhaustive pattern per type, so a field
/// added to a component fails to compile until it is listed here
pub fn expand_module() -> TokenStream {
    let mut modules = TokenStream::new();
    let mut all = Vec::new();
    for tag in TAGS {
        let name = Ident::new(tag.name, Span::call_site());
        let doc = format!("`<{}>`: {}", tag.name, tag.doc);
        let consts = tag.attrs.iter().map(|attr| {
            let ident = Ident::new(attr.name, Span::call_site());
            let doc = attr.doc;
            quote! {
                #[doc = #doc]
                pub struct #ident;
            }
        });
        let ty = Ident::new(tag.ty, Span::call_site());
        let fields = tag
            .attrs
            .iter()
            .filter(|attr| attr.field)
            .map(|attr| attr.name)
            .chain(tag.internal.iter().copied())
            .map(|field| Ident::new(field, Span::call_site()));
        let check = format_ident!("__fields_{}", tag.name);
        modules.extend(quote! {
            #[doc = #doc]
            #[allow(non_snake_case, non_camel_case_types)]
            pub mod #name {
                #(#consts)*
            }

            #[allow(dead_code, non_snake_case)]
            fn #check(value: crate::types::#ty) {
                let crate::types::#ty { #(#fields: _,)* } = value;
            }
        });
        for alias in tag.aliases {
            let alias = Ident::new(alias, Span::call_site());
            modules.extend(quote! {
                #[allow(non_snake_case)]
                pub use #name as #alias;
            });
        }
        let attrs = tag.attrs.iter().map(|attr| attr.name);
        let tag_name = tag.name;
        all.push(quote!((#tag_name, &[#(#attrs),*])));
    }
    quote! {
        /// Every built-in `rsx!` tag (a module) and its attributes (consts), for rust-analyzer to
        /// complete and document. Generated from `composure_macros`'s table.
        #[doc(hidden)]
        pub mod tags {
            #modules

            /// Each tag and its attributes (hidden, so completion offers only the tags)
            #[doc(hidden)]
            pub const ALL: &[(&str, &[&str])] = &[#(#all),*];
        }
    }
}
