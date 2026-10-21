//! `rsx!` and `#[view]`: JSX markup (parsed with rstml) expanded to `types::*` components with
//! no runtime tree, and inline handlers hoisted into the View's `ViewHandler` (ADR 0011).

use std::collections::HashSet;

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote, quote_spanned};
use rstml::node::{
    KVAttributeValue, KeyedAttributeValue, Node, NodeAttribute, NodeBlock, NodeElement, NodeName,
    NodeNameFragment,
};
use syn::{
    Expr, ExprClosure, FnArg, Ident, ItemFn, LitStr, Pat, PathArguments, ReturnType, Stmt, Type,
    TypeParamBound,
    spanned::Spanned,
    visit::{self, Visit},
    visit_mut::{self, VisitMut},
};

use crate::{
    handler::{ctx_state, is_ctx},
    meta::snake_case,
    tags::{self, TagDef},
};

fn cmd() -> TokenStream {
    quote!(::composure::command)
}

fn types() -> TokenStream {
    quote!(::composure::types)
}

fn err<T>(span: Span, message: impl std::fmt::Display) -> syn::Result<T> {
    Err(syn::Error::new(span, message))
}

/// `rsx! { .. }` outside a `#[view]`: no inline handlers. Expands what parsed even with errors,
/// so the code around and inside it still type-checks
pub fn expand_macro(input: TokenStream) -> TokenStream {
    let mut gen_ = Gen::new(None);
    let view = gen_.view(input);
    let errors = gen_.errors.into_compile_error();
    quote! {{
        #errors
        #view
    }}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Event {
    Click,
    Select,
    Submit,
}

impl Event {
    fn name(self) -> &'static str {
        match self {
            Event::Click => "click",
            Event::Select => "select",
            Event::Submit => "submit",
        }
    }
}

/// The `#[view]` function the markup is in
struct Scope {
    name: Ident,
    prefix: String,
    props: Vec<Prop>,
    modal: bool,
}

/// An inline handler, hoisted into the View's `ViewHandler`
struct Handler {
    name: String,
    event: Event,
    captured: Vec<(Ident, Type)>,
    ctx_pat: Option<Pat>,
    ctx_ty: Option<Type>,
    /// Select values or modal fields, after `ctx`
    typed: Vec<(Ident, Type)>,
    body: Expr,
    asyncness: bool,
}

/// The submit handler's typed parameters, while generating a `<Modal>`'s inputs
struct ModalScope {
    params: Vec<(Ident, Type, bool)>,
    /// An inline `on:submit`: every input needs a parameter and every parameter an input
    strict: bool,
}

struct Gen {
    scope: Option<Scope>,
    handlers: Vec<Handler>,
    names: HashSet<String>,
    /// User Views placed by the markup
    views: Vec<syn::Path>,
    /// Per `rsx!` invocation: `let`s computing inline handler ids from Props, run first so the
    /// markup can move Props
    pre: Vec<Vec<TokenStream>>,
    modal: Option<ModalScope>,
    next: usize,
    /// Every error so far: a node with an error is skipped, its siblings still generated
    errors: Errors,
    /// `&mut MarkupRoot` expressions: the invocation's root, then each `Children` level's, so
    /// the innermost level reaches it
    roots: Vec<TokenStream>,
    /// Names the `#[view]` body binds, which hoisted inline handlers cannot see
    locals: HashSet<String>,
    /// Names bound in the `{..}` blocks around the markup being expanded
    enclosing: Vec<HashSet<String>>,
    /// The invocation being expanded had parse errors: markup still being typed
    incomplete: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Parent {
    Top,
    Container,
    Section,
    Row,
    Modal,
    /// A user View's children
    Any,
}

impl Parent {
    fn takes_text(self) -> bool {
        !matches!(self, Parent::Row)
    }

    fn describe(self) -> &'static str {
        match self {
            Parent::Top => "at the top of a message",
            Parent::Container => "in a <Container>",
            Parent::Section => "in a <Section>",
            Parent::Row => "in a <Row>",
            Parent::Modal => "in a <Modal>",
            Parent::Any => "here",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tag {
    Container,
    Section,
    Text,
    Thumbnail,
    Gallery,
    Item,
    File,
    Separator,
    Row,
    Button,
    StringSelect,
    UserSelect,
    RoleSelect,
    MentionableSelect,
    ChannelSelect,
    Option,
    Modal,
    Label,
    TextInput,
    RadioGroup,
    CheckboxGroup,
    Checkbox,
    FileUpload,
}

impl Tag {
    fn parse(name: &str) -> Option<Tag> {
        Some(match name {
            "Container" => Tag::Container,
            "Section" => Tag::Section,
            "Text" | "TextDisplay" => Tag::Text,
            "Thumbnail" => Tag::Thumbnail,
            "Gallery" | "MediaGallery" => Tag::Gallery,
            "Item" | "MediaGalleryItem" => Tag::Item,
            "File" => Tag::File,
            "Separator" => Tag::Separator,
            "Row" | "ActionRow" => Tag::Row,
            "Button" => Tag::Button,
            "StringSelect" | "Select" => Tag::StringSelect,
            "UserSelect" => Tag::UserSelect,
            "RoleSelect" => Tag::RoleSelect,
            "MentionableSelect" => Tag::MentionableSelect,
            "ChannelSelect" => Tag::ChannelSelect,
            "Option" => Tag::Option,
            "Modal" => Tag::Modal,
            "Label" => Tag::Label,
            "TextInput" => Tag::TextInput,
            "RadioGroup" => Tag::RadioGroup,
            "CheckboxGroup" => Tag::CheckboxGroup,
            "Checkbox" => Tag::Checkbox,
            "FileUpload" => Tag::FileUpload,
            _ => return None,
        })
    }

    /// Its entry in the tag table
    fn def(self) -> &'static TagDef {
        tags::find(&format!("{self:?}")).expect("every tag is in the table")
    }

    fn is_select(self) -> bool {
        matches!(
            self,
            Tag::StringSelect
                | Tag::UserSelect
                | Tag::RoleSelect
                | Tag::MentionableSelect
                | Tag::ChannelSelect
        )
    }

    fn is_input(self) -> bool {
        self.is_select()
            || matches!(
                self,
                Tag::TextInput
                    | Tag::RadioGroup
                    | Tag::CheckboxGroup
                    | Tag::Checkbox
                    | Tag::FileUpload
            )
    }

    /// Whether the tag can be placed directly in `parent`
    fn allowed(self, parent: Parent) -> bool {
        let layout = matches!(
            self,
            Tag::Container
                | Tag::Text
                | Tag::Section
                | Tag::Gallery
                | Tag::File
                | Tag::Separator
                | Tag::Row
        );
        match parent {
            // A sub-View may be a single button or select meant for a row
            Parent::Top | Parent::Any => layout || self == Tag::Button || self.is_select(),
            Parent::Container => layout && self != Tag::Container,
            Parent::Section => self == Tag::Text,
            Parent::Row => self == Tag::Button || self.is_select(),
            Parent::Modal => matches!(self, Tag::Text | Tag::Label),
        }
    }

    fn placement(self) -> &'static str {
        match self {
            Tag::Button => "`<Button>` goes in a `<Row>`, or in a `<Section>` as its accessory",
            Tag::Thumbnail => "`<Thumbnail>` goes in a `<Section>` as its accessory",
            Tag::Item => "`<Item>` goes in a `<Gallery>`",
            Tag::Option => "`<Option>` goes in a select, `<RadioGroup>` or `<CheckboxGroup>`",
            Tag::Modal => "`<Modal>` must be the only top-level tag of the markup",
            Tag::Label => "`<Label>` goes in a `<Modal>`",
            Tag::Container => "`<Container>` goes at the top of a message",
            tag if tag.is_select() => "selects go in a `<Row>`, or in a `<Label>` in a modal",
            tag if tag.is_input() => "inputs go in a `<Label>` in a `<Modal>`",
            _ => "it goes in a message, a `<Container>` or a `<Section>`",
        }
    }
}

/// One attribute: its `:`-separated key and its value (`None` for a flag)
struct Attr {
    key: Vec<Ident>,
    value: Option<Expr>,
    span: Span,
}

impl Attr {
    fn is(&self, name: &str) -> bool {
        self.key.len() == 1 && self.key[0] == name
    }

    fn name(&self) -> String {
        self.key
            .iter()
            .map(Ident::to_string)
            .collect::<Vec<_>>()
            .join(":")
    }

    /// The value, `true` for a flag
    fn value_or_true(&self) -> TokenStream {
        match &self.value {
            Some(value) => value.to_token_stream(),
            None => quote_spanned!(self.span=> true),
        }
    }
}

/// An element's attributes, taken by name as the element is generated; the rest are assigned to
/// the component's fields
struct Attrs {
    list: Vec<Attr>,
    tag: String,
    /// The built-in tag's table entry; `None` for a sub-View
    def: Option<&'static TagDef>,
    /// `let _ = ::composure::command::tags::Tag::attr;` at each attribute's span, for the IDE to
    /// complete and document attribute names (taken by the element being generated)
    refs: TokenStream,
}

impl Attrs {
    /// The element's attributes, or every malformed one's error, combined
    fn new(el: &NodeElement<rstml::Infallible>, tag: &str) -> syn::Result<Self> {
        let mut list: Vec<Attr> = Vec::new();
        let mut errors = Errors::default();
        for attribute in el.attributes() {
            let attr = (|| {
                let keyed = match attribute {
                    NodeAttribute::Attribute(keyed) => keyed,
                    NodeAttribute::Block(block) => {
                        return err(block.span(), "attributes are `name=value` or `name`");
                    }
                };
                let key = attr_key(&keyed.key)?;
                let span = keyed.key.span();
                let value = match &keyed.possible_value {
                    KeyedAttributeValue::None => None,
                    KeyedAttributeValue::Value(value) => match &value.value {
                        KVAttributeValue::Expr(expr) => Some(unwrap_braces(expr)),
                        KVAttributeValue::InvalidBraced(block) => {
                            return err(block.brace.span.join(), "expected a Rust expression");
                        }
                    },
                    KeyedAttributeValue::Binding(binding) => {
                        return err(binding.paren.span.join(), "expected `name=value`");
                    }
                };
                let attr = Attr { key, value, span };
                if list.iter().any(|other| other.name() == attr.name()) {
                    return err(span, format!("duplicate attribute `{}`", attr.name()));
                }
                Ok(attr)
            })();
            match attr {
                Ok(attr) => list.push(attr),
                Err(error) => errors.push(error),
            }
        }
        errors.finish()?;
        let def = tags::find(tag);
        let mut refs = TokenStream::new();
        if let Some(def) = def {
            let module = Ident::new(def.name, Span::call_site());
            for attr in &list {
                // Known names, and the start of one while it is typed (which also fails as an
                // unknown field, as it would anyway)
                let key = &attr.key[0];
                let name = key.to_string();
                if def.has(&name) || def.completes(&name) {
                    refs.extend(quote! {
                        let _ = ::composure::command::tags::#module::#key;
                    });
                }
            }
        }
        Ok(Self {
            list,
            tag: tag.to_string(),
            def,
            refs,
        })
    }

    fn take(&mut self, names: &[&str]) -> Option<Attr> {
        if let Some(def) = self.def {
            debug_assert!(
                names.iter().all(|name| def.has(name)),
                "`{names:?}` of `<{}>` missing from the tag table",
                def.name
            );
        }
        let index = self
            .list
            .iter()
            .position(|attr| names.iter().any(|name| attr.is(name)))?;
        Some(self.list.remove(index))
    }

    fn has(&self, name: &str) -> bool {
        if let Some(def) = self.def {
            debug_assert!(
                def.has(name),
                "`{name}` of `<{}>` missing from the tag table",
                def.name
            );
        }
        self.list.iter().any(|attr| attr.is(name))
    }

    /// `url` (or `src`): what `IntoMedia` turns into a URL
    fn media(&mut self) -> syn::Result<(Span, Expr)> {
        match self.value(&["url", "src"])? {
            Some(media) => Ok(media),
            None => err(
                Span::call_site(),
                format!(
                    "`<{}>` needs `url` or `src` (a URL, or a `File` to upload)",
                    self.tag
                ),
            ),
        }
    }

    fn value(&mut self, names: &[&str]) -> syn::Result<Option<(Span, Expr)>> {
        match self.take(names) {
            None => Ok(None),
            Some(Attr {
                value: Some(value),
                span,
                ..
            }) => Ok(Some((span, value))),
            Some(attr) => err(attr.span, format!("`{}` needs a value", attr.name())),
        }
    }

    fn required(&mut self, names: &[&str], what: &str) -> syn::Result<Expr> {
        match self.value(names)? {
            Some((_, value)) => Ok(value),
            None => err(
                Span::call_site(),
                format!("`<{}>` needs `{}` ({what})", self.tag, names[0]),
            ),
        }
    }

    fn flag(&mut self, name: &str) -> syn::Result<Option<Span>> {
        match self.take(&[name]) {
            None => Ok(None),
            Some(Attr {
                value: None, span, ..
            }) => Ok(Some(span)),
            Some(attr) => err(attr.span, format!("`{name}` takes no value")),
        }
    }

    fn event(&mut self) -> Option<Attr> {
        let index = self.list.iter().position(|attr| attr.key[0] == "on")?;
        Some(self.list.remove(index))
    }

    /// `#var.field = IntoAttr::into_attr(value);` for the remaining attributes, with `aliases`
    /// (`min` for `min_values`, ...)
    fn assign(self, var: &Ident, aliases: &[(&str, &str)]) -> syn::Result<TokenStream> {
        let c = cmd();
        let mut out = TokenStream::new();
        let mut errors = Errors::default();
        for attr in self.list {
            if attr.key.len() != 1 {
                errors.push(syn::Error::new(
                    attr.span,
                    format!("`<{}>` has no `{}` attribute", self.tag, attr.name()),
                ));
                continue;
            }
            let key = &attr.key[0];
            let field = match aliases.iter().find(|(alias, _)| key == alias) {
                Some((_, field)) => Ident::new(field, key.span()),
                None => key.clone(),
            };
            let value = attr.value_or_true();
            out.extend(quote_spanned! {attr.span=>
                #var.#field = #c::IntoAttr::into_attr(#value);
            });
        }
        errors.finish()?;
        Ok(out)
    }
}

/// Errors combined into one `syn::Error`, so every one is reported
#[derive(Default)]
struct Errors(Option<syn::Error>);

impl Errors {
    fn push(&mut self, error: syn::Error) {
        match &mut self.0 {
            Some(errors) => errors.combine(error),
            None => self.0 = Some(error),
        }
    }

    fn finish(self) -> syn::Result<()> {
        self.0.map_or(Ok(()), Err)
    }

    fn into_compile_error(self) -> Option<TokenStream> {
        self.0.map(syn::Error::into_compile_error)
    }
}

fn attr_key(name: &NodeName) -> syn::Result<Vec<Ident>> {
    match name {
        NodeName::Path(path) if path.path.segments.len() == 1 => {
            Ok(vec![path.path.segments[0].ident.clone()])
        }
        NodeName::Punctuated(punctuated) => {
            let mut key = Vec::new();
            for pair in punctuated.pairs() {
                if let Some(punct) = pair.punct()
                    && punct.as_char() != ':'
                {
                    return err(punct.span(), "attribute names use `_`, e.g. `accent_color`");
                }
                match pair.value() {
                    NodeNameFragment::Ident(ident) => key.push(ident.clone()),
                    _ => return err(name.span(), "expected an attribute name"),
                }
            }
            Ok(key)
        }
        _ => err(name.span(), "expected an attribute name"),
    }
}

/// `{move |ctx: ComponentCtx<impl Platform>| ..}`, `{Vec::<T>::new()}`: braces let a value use
/// `<..>` in markup; the value itself is used, so it is not linted as needless braces
fn unwrap_braces(expr: &Expr) -> Expr {
    if let Expr::Block(block) = expr
        && block.attrs.is_empty()
        && block.label.is_none()
        && let [Stmt::Expr(inner, None)] = block.block.stmts.as_slice()
    {
        return inner.clone();
    }
    expr.clone()
}

fn is_comment(node: &Node) -> bool {
    match node {
        Node::Comment(_) => true,
        Node::RawText(raw) => raw.to_token_stream().is_empty(),
        _ => false,
    }
}

fn element_name(el: &NodeElement<rstml::Infallible>) -> syn::Result<syn::Path> {
    match el.name() {
        NodeName::Path(path) => Ok(path.path.clone()),
        other => err(other.span(), "expected a tag name"),
    }
}

/// Every identifier `tokens` uses: not field labels or bindings (`name: ..`), but names inside
/// format strings
fn idents(tokens: TokenStream, out: &mut HashSet<String>) {
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    for (i, tree) in trees.iter().enumerate() {
        match tree {
            TokenTree::Ident(ident) => {
                let labeled = matches!(
                    (trees.get(i + 1), trees.get(i + 2)),
                    (Some(TokenTree::Punct(colon)), next)
                        if colon.as_char() == ':'
                            && colon.spacing() == proc_macro2::Spacing::Alone
                            && !matches!(next, Some(TokenTree::Punct(p)) if p.as_char() == ':')
                );
                if !labeled {
                    out.insert(ident.to_string());
                }
            }
            TokenTree::Group(group) => idents(group.stream(), out),
            // Inline format arguments: `format!("{id}")`
            TokenTree::Literal(literal) => {
                let text = literal.to_string();
                if text.ends_with('"') {
                    format_args(&text, out);
                }
            }
            _ => {}
        }
    }
}

fn all_idents(tokens: TokenStream, out: &mut HashSet<String>) {
    for tree in tokens {
        match tree {
            TokenTree::Ident(ident) => {
                out.insert(ident.to_string());
            }
            TokenTree::Group(group) => all_idents(group.stream(), out),
            _ => {}
        }
    }
}

/// The names in `{name}` / `{name:..}` of a string literal
fn format_args(text: &str, out: &mut HashSet<String>) {
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '{' {
            continue;
        }
        if chars.peek() == Some(&'{') {
            chars.next();
            continue;
        }
        let mut name = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                name.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if !name.is_empty() && matches!(chars.peek(), Some('}' | ':')) {
            out.insert(name);
        }
    }
}

/// The expansion of an `rsx!` that `#[view]` expanded itself, still naming the macro so its import
/// counts as used
fn used(path: &syn::Path, tokens: TokenStream) -> TokenStream {
    quote! {{
        #[allow(unused_imports)]
        use #path as _;
        #tokens
    }}
}

fn is_view_macro(path: &syn::Path) -> bool {
    path.segments.last().is_some_and(|s| s.ident == "rsx")
}

/// Expands the `rsx!` invocations in a `#[view]` body (and in markup blocks) with the View's
/// Scope, so inline handlers can be hoisted
struct Expander<'a> {
    gen_: &'a mut Gen,
}

impl VisitMut for Expander<'_> {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Macro(mac) = expr
            && is_view_macro(&mac.mac.path)
        {
            let tokens = self.gen_.view(mac.mac.tokens.clone());
            *expr = Expr::Verbatim(used(&mac.mac.path, tokens));
            return;
        }
        visit_mut::visit_expr_mut(self, expr);
    }

    fn visit_stmt_mut(&mut self, stmt: &mut Stmt) {
        if let Stmt::Macro(mac) = stmt
            && is_view_macro(&mac.mac.path)
        {
            let tokens = self.gen_.view(mac.mac.tokens.clone());
            *stmt = Stmt::Expr(Expr::Verbatim(used(&mac.mac.path, tokens)), mac.semi_token);
            return;
        }
        visit_mut::visit_stmt_mut(self, stmt);
    }
}

impl Gen {
    fn new(scope: Option<Scope>) -> Self {
        Self {
            scope,
            handlers: Vec::new(),
            names: HashSet::new(),
            views: Vec::new(),
            pre: Vec::new(),
            modal: None,
            next: 0,
            errors: Errors::default(),
            roots: Vec::new(),
            locals: HashSet::new(),
            enclosing: Vec::new(),
            incomplete: false,
        }
    }

    fn var(&mut self, name: &str) -> Ident {
        self.next += 1;
        Ident::new(&format!("__{name}{}", self.next), Span::mixed_site())
    }

    /// Records an error; generation goes on
    fn error(&mut self, error: syn::Error) {
        self.errors.push(error);
    }

    /// The value, or `None` with the error recorded
    fn recover<T>(&mut self, result: syn::Result<T>) -> Option<T> {
        result.map_err(|error| self.error(error)).ok()
    }

    /// A `&mut MarkupRoot` expression for the level being generated
    fn root(&self) -> TokenStream {
        self.roots
            .last()
            .cloned()
            .expect("inside an rsx! invocation")
    }

    /// One `rsx!` invocation: a `View`, or a `ModalView` for a `<Modal>` root. Never fails: errors
    /// are recorded and what parsed is still expanded (`View::default()` or a placeholder
    /// `ModalView` when nothing did), so `{..}` blocks keep type-checking in incomplete markup
    fn view(&mut self, tokens: TokenStream) -> TokenStream {
        let parser = rstml::Parser::new(rstml::ParserConfig::new().recover_block(true));
        let (nodes, diagnostics) = parser.parse_recoverable(tokens).split_vec();
        let incomplete = !diagnostics.is_empty();
        for diagnostic in diagnostics {
            // Each spanned child becomes its own `error:`; rstml's notes ("treating all inputs
            // after open tag as it content") only add noise, its helps point at the open tag
            let error: syn::Error = diagnostic.into();
            for part in error {
                if !part.to_string().starts_with("[note]") {
                    self.error(part);
                }
            }
        }
        let outer_incomplete = std::mem::replace(&mut self.incomplete, incomplete);
        self.pre.push(Vec::new());
        let root = self.var("root");
        self.roots.push(quote!((&mut #root)));
        let meaningful = flatten(&nodes);
        let modal = match meaningful.as_slice() {
            [Node::Element(el)] if element_name(el).is_ok_and(|path| path.is_ident("Modal")) => {
                Some(el)
            }
            _ => None,
        };
        let in_modal_view = self.scope.as_ref().is_some_and(|scope| scope.modal);
        let body = match modal {
            Some(el) => self.modal(el, &root),
            // A `#[view] -> ModalView` whose `<Modal>` is still being typed
            None if incomplete && in_modal_view => {
                let data = modal_data(
                    quote!(::composure::types::ModalCallbackData),
                    quote!(::std::string::String::new()),
                    quote!(::std::string::String::new()),
                );
                quote!(#root.modal(#data, ::std::vec::Vec::new()))
            }
            None => {
                let components = self.level(&nodes, Parent::Top);
                let v = self.var("v");
                quote! {
                    let #v = #components;
                    #root.view(#v)
                }
            }
        };
        self.roots.pop();
        self.incomplete = outer_incomplete;
        let pre = self.pre.pop().expect("pushed above");
        let c = cmd();
        quote! {{
            #(#pre)*
            #[allow(unused_mut)]
            let mut #root = #c::MarkupRoot::default();
            #body
        }}
    }

    /// A `{..}` in the markup; nested `rsx!`s share this View's Scope. An invalid block (`{x.}`,
    /// kept by `recover_block`) is passed through for rustc to report and the IDE to complete
    fn block(&mut self, block: &NodeBlock) -> TokenStream {
        let NodeBlock::ValidBlock(block) = block else {
            return block.to_token_stream();
        };
        let mut block = block.clone();
        if self.scope.is_some() {
            // Names bound in the block (`|item| rsx! { .. }`) are not visible to the inline
            // handlers of the markup inside it
            self.enclosing.push(bindings(|v| v.visit_block(&block)));
            let mut expander = Expander { gen_: self };
            expander.visit_block_mut(&mut block);
            self.enclosing.pop();
        }
        // `{expr}` as the expression itself, so it is not linted as needless braces
        if let [Stmt::Expr(expr, None)] = block.stmts.as_slice() {
            return quote!((#expr));
        }
        block.to_token_stream()
    }

    /// The components of one markup level, a `Vec<Component>` expression: `vec![..]` when every
    /// child is known at compile time (literal text folded), else collected through `Children`.
    /// A node with an error is recorded and skipped.
    fn level<'n>(
        &mut self,
        nodes: impl IntoIterator<Item = &'n Node>,
        parent: Parent,
    ) -> TokenStream {
        let c = cmd();
        let t = types();
        let nodes = flatten(nodes);
        let pieces = pieces(&nodes);
        let fixed = nodes.iter().all(|node| match node {
            Node::Block(_) => false,
            Node::Element(el) => !matches!(classify(el), Ok(Kind::View(_))),
            _ => true,
        });
        if fixed {
            self.check_counts(&pieces, parent);
        }
        if fixed {
            let mut items = Vec::new();
            for piece in &pieces {
                match piece {
                    Piece::Text(lits) => {
                        if let Some(text) = self.text_run(lits, parent) {
                            items.push(quote!(#t::Component::text(#text)));
                        }
                    }
                    Piece::Node(node) => {
                        if let Some(Child::Component(component)) = self.child(node, parent) {
                            items.push(component);
                        }
                    }
                }
            }
            return quote!(::std::vec![#(#items),*]);
        }
        let cx = self.var("c");
        let root = self.root();
        self.roots.push(quote!(#cx.root()));
        let mut body = TokenStream::new();
        for piece in &pieces {
            match piece {
                Piece::Text(lits) => {
                    if let Some(text) = self.text_run(lits, parent) {
                        body.extend(quote!(#cx.text(#text);));
                    }
                }
                Piece::Node(node) => match self.child(node, parent) {
                    Some(Child::Component(component)) => {
                        let e = self.var("e");
                        body.extend(quote! {
                            let #e = #component;
                            #cx.component(#e);
                        });
                    }
                    Some(Child::Push(value)) => {
                        body.extend(quote!(#c::IntoChildren::push_into(#value, &mut #cx);));
                    }
                    None => {}
                },
            }
        }
        self.roots.pop();
        quote! {{
            #[allow(unused_mut)]
            let mut #cx = #c::Children::new(#root);
            #body
            #cx.finish()
        }}
    }

    /// Adjacent literals as one, or `None` (recorded) where text cannot go
    fn text_run(&mut self, lits: &[&LitStr], parent: Parent) -> Option<LitStr> {
        if !parent.takes_text() {
            self.error(syn::Error::new(
                lits[0].span(),
                format!("text cannot go {}", parent.describe()),
            ));
            return None;
        }
        Some(fold(lits))
    }

    /// The counts Discord limits that a level known at compile time shows (ADR 0015); the
    /// rest are checked when the View is answered with
    fn check_counts(&mut self, pieces: &[Piece], parent: Parent) {
        let span = |piece: &Piece| match piece {
            Piece::Text(lits) => lits[0].span(),
            Piece::Node(Node::Element(el)) => el.name().span(),
            Piece::Node(node) => node.span(),
        };
        let (max, message) = match parent {
            Parent::Row => {
                let selects = pieces.iter().position(|piece| {
                    matches!(piece, Piece::Node(Node::Element(el))
                        if matches!(classify(el), Ok(Kind::Tag(tag)) if tag.is_select()))
                });
                if let (Some(_), Some(second)) = (selects, pieces.get(1)) {
                    self.error(syn::Error::new(
                        span(second),
                        "a select fills its `<Row>`: nothing else can go beside it",
                    ));
                    return;
                }
                (limits::MAX_ROW_BUTTONS, "a `<Row>` holds up to 5 buttons")
            }
            Parent::Section => (
                limits::SECTION_TEXTS_MAX,
                "a `<Section>` holds 1-3 texts beside its accessory",
            ),
            Parent::Modal => (
                limits::MODAL_COMPONENTS_MAX,
                "a `<Modal>` holds up to 5 components",
            ),
            _ => return,
        };
        if let Some(extra) = pieces.get(max) {
            self.error(syn::Error::new(span(extra), message));
        }
    }

    /// One node of a level, as a component or a value to push; `None` for nothing (or an error,
    /// recorded)
    fn child(&mut self, node: &Node, parent: Parent) -> Option<Child> {
        let result = match node {
            Node::RawText(raw) if !raw.to_token_stream().is_empty() => {
                err(raw.span(), "put text in quotes: \"text\"")
            }
            Node::Block(block) => Ok(Some(Child::Push(self.block(block)))),
            Node::Element(el) => match classify(el) {
                Ok(Kind::Tag(tag)) => self
                    .element(el, tag, parent)
                    .map(|component| Some(Child::Component(component))),
                Ok(Kind::View(path)) => {
                    self.user_view(el, path).map(|view| Some(Child::Push(view)))
                }
                Err(error) => Err(error),
            },
            Node::Doctype(doctype) => err(doctype.span(), "unexpected doctype"),
            _ => Ok(None),
        };
        self.recover(result).flatten()
    }

    /// A built-in tag where it is placed: a `Component` expression
    fn element(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        tag: Tag,
        parent: Parent,
    ) -> syn::Result<TokenStream> {
        if !tag.allowed(parent) {
            return err(el.name().span(), tag.placement());
        }
        self.component(el, tag)
    }

    /// An expression building the component for a built-in tag
    fn component(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        tag: Tag,
    ) -> syn::Result<TokenStream> {
        let c = cmd();
        let t = types();
        let name = format!("{tag:?}");
        let mut attrs = Attrs::new(el, &name)?;
        let e = self.var("e");
        let span = el.name().span();
        let ty = tag_type(tag, el, false);
        let close = close_type(tag, el, false);
        let refs = std::mem::take(&mut attrs.refs);
        let no_children = |el: &NodeElement<rstml::Infallible>| -> syn::Result<()> {
            match el.children.iter().find(|node| !is_comment(node)) {
                Some(child) => err(child.span(), format!("`<{name}>` takes no children")),
                None => Ok(()),
            }
        };
        let r = self.root();
        Ok(match tag {
            Tag::Container | Tag::Row => {
                let (inner, variant) = match tag {
                    Tag::Container => (Parent::Container, quote!(Container)),
                    _ => (Parent::Row, quote!(ActionRow)),
                };
                let components = self.level(&el.children, inner);
                let assign = attrs.assign(&e, &[])?;
                quote! {{
                    #refs
                    #close
                    #[allow(unused_mut)]
                    let mut #e = #ty::new(#components);
                    #assign
                    #t::Component::#variant(#e)
                }}
            }
            Tag::Text => {
                let content = match attrs.value(&["content"])? {
                    Some((_, value)) => quote!(#value),
                    None => self
                        .text(&el.children, "Text")
                        .unwrap_or_else(|| quote!(::std::string::String::new())),
                };
                let assign = attrs.assign(&e, &[])?;
                quote! {{
                    #refs
                    #close
                    #[allow(unused_mut)]
                    let mut #e = #ty::new(#content);
                    #assign
                    #t::Component::TextDisplay(#e)
                }}
            }
            Tag::Section => self.section(el, attrs, ty, close, refs)?,
            Tag::Thumbnail | Tag::File => {
                no_children(el)?;
                let (url_span, url) = attrs.media()?;
                let assign = attrs.assign(&e, &[])?;
                let variant = match tag {
                    Tag::Thumbnail => quote!(Thumbnail),
                    _ => quote!(File),
                };
                let media = quote_spanned!(url_span=> #c::IntoMedia::into_media(#url, #r));
                quote! {{
                    #refs
                    #close
                    #[allow(unused_mut)]
                    let mut #e = #ty::new(#media);
                    #assign
                    #t::Component::#variant(#e)
                }}
            }
            Tag::Separator => {
                no_children(el)?;
                let spacing = match (attrs.flag("small")?, attrs.flag("large")?) {
                    (Some(_), Some(span)) => {
                        return err(span, "a separator is `small` or `large`");
                    }
                    (Some(_), None) => quote! {
                        #e.spacing = ::core::option::Option::Some(#t::SeparatorSpacing::Small);
                    },
                    (None, Some(_)) => quote! {
                        #e.spacing = ::core::option::Option::Some(#t::SeparatorSpacing::Large);
                    },
                    (None, None) => quote!(),
                };
                let assign = attrs.assign(&e, &[])?;
                quote! {{
                    #refs
                    #close
                    #[allow(unused_mut)]
                    let mut #e = #ty::new();
                    #spacing
                    #assign
                    #t::Component::Separator(#e)
                }}
            }
            Tag::Gallery => {
                let items = self.gallery_items(el);
                let assign = attrs.assign(&e, &[])?;
                quote! {{
                    #refs
                    #close
                    #[allow(unused_mut)]
                    let mut #e = #ty::new(#items);
                    #assign
                    #t::Component::MediaGallery(#e)
                }}
            }
            Tag::Button => self.button(el, attrs, ty, close, refs)?,
            tag if tag.is_select() => self.select(el, tag, attrs, ty, close, refs)?,
            Tag::Label => self.label(el, attrs, ty, close, refs)?,
            tag if tag.is_input() => self.input(el, tag, attrs, ty, close, refs)?,
            Tag::Item | Tag::Option | Tag::Modal => return err(span, tag.placement()),
            _ => unreachable!("every tag is handled"),
        })
    }

    /// The items of a `<Gallery>`, a `Vec<MediaGalleryItem>` expression
    fn gallery_items(&mut self, el: &NodeElement<rstml::Infallible>) -> TokenStream {
        let t = types();
        let nodes = flatten(&el.children);
        let mut items = Vec::new();
        let mut dynamic = false;
        let mut count = 0;
        for node in nodes {
            let item = match node {
                Node::Element(item)
                    if element_name(item).is_ok_and(|path| {
                        path.is_ident("Item") || path.is_ident("MediaGalleryItem")
                    }) =>
                {
                    count += 1;
                    if count == limits::GALLERY_ITEMS_MAX + 1 {
                        self.error(syn::Error::new(
                            item.name().span(),
                            "a `<Gallery>` holds up to 10 items",
                        ));
                    }
                    self.item(item).map(Item::One)
                }
                Node::Block(block) => {
                    dynamic = true;
                    Ok(Item::Many(self.block(block)))
                }
                other => err(
                    other.span(),
                    "a `<Gallery>` takes `<Item url=..>`s and `{..}` of `MediaGalleryItem`s",
                ),
            };
            items.extend(self.recover(item));
        }
        collect_items(items, dynamic, quote!(#t::MediaGalleryItem))
    }

    /// The text of a text-only element (`<Text>`, a button label, an option), or `None` without
    /// children; a child that is not text is recorded and skipped. Literals are joined at compile
    /// time: only literals become one `String::from`, else the `String` is sized for them.
    fn text(&mut self, nodes: &[Node], tag: &str) -> Option<TokenStream> {
        let c = cmd();
        let mut parts: Vec<TextPart> = Vec::new();
        for node in nodes {
            match node {
                Node::Text(lit) => match parts.last_mut() {
                    Some(TextPart::Lits(lits)) => lits.push(&lit.value),
                    _ => parts.push(TextPart::Lits(vec![&lit.value])),
                },
                Node::Block(block) => parts.push(TextPart::Expr(self.block(block))),
                Node::RawText(raw) if !raw.to_token_stream().is_empty() => {
                    self.error(syn::Error::new(raw.span(), "put text in quotes: \"text\""));
                }
                Node::Element(el) => {
                    self.error(syn::Error::new(
                        el.name().span(),
                        format!("`<{tag}>` only takes text"),
                    ));
                }
                _ => {}
            }
        }
        match parts.as_slice() {
            [] => None,
            [TextPart::Lits(lits)] => {
                let text = fold(lits);
                Some(quote!(::std::string::String::from(#text)))
            }
            _ => {
                let text = self.var("t");
                let capacity: usize = parts
                    .iter()
                    .map(|part| match part {
                        TextPart::Lits(lits) => lits.iter().map(|lit| lit.value().len()).sum(),
                        // Room for a number or a short name, so most texts allocate once
                        TextPart::Expr(_) => EXPR_CAPACITY,
                    })
                    .sum();
                let pushes = parts.iter().map(|part| match part {
                    TextPart::Lits(lits) => {
                        let lit = fold(lits);
                        quote!(#text.push_str(#lit);)
                    }
                    TextPart::Expr(expr) => quote!(#c::IntoText::push_text(#expr, &mut #text);),
                });
                Some(quote! {{
                    let mut #text = ::std::string::String::with_capacity(#capacity);
                    #(#pushes)*
                    #text
                }})
            }
        }
    }

    fn section(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        mut attrs: Attrs,
        ty: TokenStream,
        close: TokenStream,
        refs: TokenStream,
    ) -> syn::Result<TokenStream> {
        let t = types();
        // `Some(None)`: the accessory child had an error, recorded
        let mut accessory: Option<Option<TokenStream>> = None;
        let mut rest = Vec::new();
        for node in flatten(&el.children) {
            if let Node::Element(child) = node
                && let Ok(path) = element_name(child)
                && (path.is_ident("Button") || path.is_ident("Thumbnail"))
            {
                if accessory.is_some() {
                    self.error(syn::Error::new(
                        child.name().span(),
                        "a `<Section>` has one accessory: a `<Button>` or `<Thumbnail>`",
                    ));
                    continue;
                }
                let tag =
                    Tag::parse(&path.get_ident().expect("an ident").to_string()).expect("a tag");
                let component = self.component(child, tag);
                accessory = Some(self.recover(component));
                continue;
            }
            rest.push(node);
        }
        let accessory = match (accessory, attrs.value(&["accessory"])?) {
            (Some(None), _) => placeholder(),
            (Some(Some(accessory)), None) => accessory,
            (Some(_), Some((span, _))) => {
                return err(
                    span,
                    "the accessory is already a `<Button>` or `<Thumbnail>` child",
                );
            }
            (None, Some((_, value))) => {
                quote!(::core::convert::Into::<#t::Component>::into(#value))
            }
            (None, None) => {
                return err(
                    el.name().span(),
                    "a `<Section>` needs an accessory: a `<Button>` or `<Thumbnail>` child",
                );
            }
        };
        let components = self.level(rest, Parent::Section);
        let e = self.var("e");
        let inner = self.var("c");
        let acc = self.var("accessory");
        let assign = attrs.assign(&e, &[])?;
        Ok(quote! {{
            #refs
            #close
            let #inner = #components;
            let #acc = #accessory;
            #[allow(unused_mut)]
            let mut #e = #ty::new(#inner, #acc);
            #assign
            #t::Component::Section(#e)
        }})
    }

    fn item(&mut self, el: &NodeElement<rstml::Infallible>) -> syn::Result<TokenStream> {
        let c = cmd();
        let mut attrs = Attrs::new(el, "Item")?;
        let refs = std::mem::take(&mut attrs.refs);
        let ty = tag_type(Tag::Item, el, false);
        let close = close_type(Tag::Item, el, false);
        let (url_span, url) = attrs.media()?;
        let e = self.var("e");
        let r = self.root();
        let media = quote_spanned!(url_span=> #c::IntoMedia::into_media(#url, #r));
        let assign = attrs.assign(&e, &[])?;
        Ok(quote! {{
            #refs
            #close
            #[allow(unused_mut)]
            let mut #e = #ty::new(#media);
            #assign
            #e
        }})
    }

    fn button(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        mut attrs: Attrs,
        ty: TokenStream,
        close: TokenStream,
        refs: TokenStream,
    ) -> syn::Result<TokenStream> {
        let c = cmd();
        let t = types();
        let mut style_flag = None;
        for (flag, variant) in [
            ("primary", "Primary"),
            ("secondary", "Secondary"),
            ("success", "Success"),
            ("danger", "Danger"),
        ] {
            if let Some(span) = attrs.flag(flag)? {
                if style_flag.is_some() {
                    return err(span, "a button has one style");
                }
                style_flag = Some(Ident::new(variant, span));
            }
        }
        let style_attr = attrs.value(&["style"])?;
        let link = attrs.has("url");
        let premium = attrs.has("sku_id");
        let event = attrs.event();
        let custom_id = attrs.value(&["custom_id"])?;
        let label_attr = attrs.value(&["label"])?;
        let label_text = self.text(&el.children, "Button");
        let span = el.name().span();

        if (link || premium) && (event.is_some() || custom_id.is_some()) {
            return err(
                span,
                "a link or premium button cannot have `on:click` or a `custom_id`",
            );
        }
        if !link && !premium && event.is_none() && custom_id.is_none() {
            return err(
                span,
                "a `<Button>` needs `on:click`, a `custom_id`, or a `url`",
            );
        }
        let label = match (label_attr, label_text) {
            (Some((span, _)), Some(_)) => {
                return err(span, "the label is either `label` or the button's text");
            }
            (Some((_, value)), None) => {
                quote!(#c::IntoAttr::<::core::option::Option<::std::string::String>>::into_attr(#value))
            }
            (None, Some(text)) => quote!(::core::option::Option::Some(#text)),
            (None, None) => quote!(::core::option::Option::None),
        };
        let style = match (style_flag, style_attr) {
            (Some(_), Some((span, _))) => return err(span, "a button has one style"),
            (Some(variant), None) => quote!(#t::ButtonStyle::#variant),
            (None, Some((_, value))) => quote!(#c::IntoAttr::<#t::ButtonStyle>::into_attr(#value)),
            (None, None) if link => quote!(#t::ButtonStyle::Link),
            (None, None) if premium => quote!(#t::ButtonStyle::Premium),
            (None, None) => quote!(#t::ButtonStyle::Secondary),
        };
        let custom_id = match (event, custom_id) {
            (Some(_), Some((span, _))) => {
                return err(span, "the custom_id comes from `on:click`");
            }
            (Some(event), None) => {
                let (id, _) = self.event(event, Event::Click, "Button")?;
                quote!(::core::option::Option::Some(#id))
            }
            (None, Some((_, value))) => {
                quote!(#c::IntoAttr::<::core::option::Option<::std::string::String>>::into_attr(#value))
            }
            (None, None) => quote!(::core::option::Option::None),
        };
        let e = self.var("e");
        let assign = attrs.assign(&e, &[])?;
        Ok(quote! {{
            #refs
            #close
            #[allow(unused_mut)]
            let mut #e = #ty::new(
                #style,
                #label,
                ::core::option::Option::None,
                #custom_id,
                ::core::option::Option::None,
                ::core::option::Option::None,
            );
            #assign
            #t::Component::Button(#e)
        }})
    }

    /// The `custom_id` (a `String` expression) for an `on:<event>` attribute: a reference to a
    /// handler (`on:click=like(likes)`), or an inline handler (`on:click:like=|ctx| ..`), hoisted.
    /// Returns the inline handler's index.
    fn event(
        &mut self,
        attr: Attr,
        expected: Event,
        tag: &str,
    ) -> syn::Result<(TokenStream, Option<usize>)> {
        let event = attr.key.get(1).map(Ident::to_string);
        if event.as_deref() != Some(expected.name()) {
            return err(
                attr.span,
                format!("`<{tag}>` answers `on:{}`", expected.name()),
            );
        }
        let key_name = attr.name();
        let Some(value) = attr.value else {
            return err(attr.span, format!("`{key_name}` needs a handler"));
        };
        let r = self.root();
        match attr.key.len() {
            2 => {
                let custom_id = match &value {
                    Expr::Call(call) => {
                        let func = &call.func;
                        let args = &call.args;
                        quote!(#func::custom_id(#args))
                    }
                    Expr::Path(path) => quote!(#path::custom_id()),
                    other => {
                        return err(
                            other.span(),
                            format!(
                                "expected a handler with its state, like `like(likes)`, or an inline handler: `on:{}:name=|ctx| ..`",
                                expected.name()
                            ),
                        );
                    }
                };
                Ok((quote!(#r.id(#custom_id)), None))
            }
            3 => {
                let Expr::Closure(closure) = value else {
                    return err(
                        value.span(),
                        format!(
                            "an inline handler is a closure: `on:{}:{}=move |ctx| ..` (in braces if its types use `<..>`)",
                            expected.name(),
                            attr.key[2]
                        ),
                    );
                };
                let var = self.inline(attr.key[2].clone(), expected, closure)?;
                Ok((quote!(#r.id(#var)), Some(self.handlers.len() - 1)))
            }
            _ => err(attr.span, "expected `on:event` or `on:event:name`"),
        }
    }

    /// Registers an inline handler and returns the variable holding its `custom_id`
    fn inline(&mut self, name: Ident, event: Event, closure: ExprClosure) -> syn::Result<Ident> {
        let Some(scope) = &self.scope else {
            return err(
                name.span(),
                "inline handlers need the markup to be in a `#[view]` function",
            );
        };
        if scope.modal != (event == Event::Submit) {
            return err(
                name.span(),
                "`on:submit` goes on a modal View's `<Modal>`, `on:click` and `on:select` in message Views",
            );
        }
        let handler_name = name.to_string();
        if !self.names.insert(handler_name.clone()) {
            return err(
                name.span(),
                format!(
                    "`{}` already has a handler named `{handler_name}`",
                    scope.name
                ),
            );
        }

        let ctx_name = match event {
            Event::Submit => "ModalCtx",
            _ => "ComponentCtx",
        };
        let mut inputs = closure.inputs.iter();
        let (ctx_pat, ctx_ty) = match inputs.next() {
            None => (None, None),
            Some(Pat::Type(typed)) => {
                if !is_ctx(&typed.ty, ctx_name) {
                    return err(
                        typed.ty.span(),
                        format!("the first parameter is the `{ctx_name}`"),
                    );
                }
                (Some((*typed.pat).clone()), Some((*typed.ty).clone()))
            }
            Some(pat) => (Some(pat.clone()), None),
        };
        let mut typed = Vec::new();
        for input in inputs {
            let Pat::Type(param) = input else {
                return err(input.span(), "give the parameter a type: `name: Type`");
            };
            let Pat::Ident(ident) = &*param.pat else {
                return err(param.pat.span(), "parameters must be plain names");
            };
            typed.push((ident.ident.clone(), (*param.ty).clone()));
        }
        match event {
            Event::Click if !typed.is_empty() => {
                return err(typed[0].0.span(), "`on:click` handlers take only `ctx`");
            }
            Event::Select if typed.len() > 1 => {
                return err(
                    typed[1].0.span(),
                    "`on:select` handlers take `ctx` and the selected values",
                );
            }
            _ => {}
        }

        let mut used = HashSet::new();
        idents(closure.body.to_token_stream(), &mut used);
        let mut params = HashSet::new();
        for input in &closure.inputs {
            let pat = match input {
                Pat::Type(typed) => &*typed.pat,
                other => other,
            };
            all_idents(pat.to_token_stream(), &mut params);
        }
        self.check_locals(&closure, &params)?;
        let scope = self.scope.as_ref().expect("checked above");
        let mut captured = Vec::new();
        for prop in &scope.props {
            let prop_name = prop.name.to_string();
            if used.contains(&prop_name) && !params.contains(&prop_name) {
                if prop.kind == PropKind::Children {
                    return err(
                        name.span(),
                        "handlers cannot use `children`: it is not carried in the custom_id",
                    );
                }
                captured.push((prop.name.clone(), prop.ty.clone()));
            }
        }

        let c = cmd();
        let full = format!("{}:{handler_name}", scope.prefix);
        let var = self.var("id");
        let names = captured.iter().map(|(name, _)| name);
        self.pre.last_mut().expect("inside a view").push(quote! {
            let #var = #c::custom_id::encode(#full, &(#(&#names,)*));
        });
        let asyncness = closure.asyncness.is_some();
        self.handlers.push(Handler {
            name: handler_name,
            event,
            captured,
            ctx_pat,
            ctx_ty,
            typed,
            body: *closure.body,
            asyncness,
        });
        Ok(var)
    }

    /// An inline handler is hoisted into the View's `ViewHandler`, where only Props (decoded from
    /// the `custom_id`) exist: a name the `#[view]` body or a closure around the markup binds is
    /// an error at its use, instead of rustc's "cannot find value" in code the user never wrote
    fn check_locals(&self, closure: &ExprClosure, params: &HashSet<String>) -> syn::Result<()> {
        let scope = self.scope.as_ref().expect("inline handlers are in a View");
        let inside = bindings(|v| v.visit_expr(&closure.body));
        let mut errors = Errors::default();
        let mut reported = HashSet::new();
        for ident in uses(&closure.body) {
            let name = ident.to_string();
            let local = self.locals.contains(&name)
                || self.enclosing.iter().any(|names| names.contains(&name));
            if !local || params.contains(&name) || inside.contains(&name) {
                continue;
            }
            if !reported.insert(name.clone()) {
                continue;
            }
            let message = if scope.props.iter().any(|prop| prop.name == name) {
                format!(
                    "a local `{name}` shadows the Prop `{name}`, but inline handlers only see Props (carried in the custom_id), so this is the Prop. Rename the local, or make the value a Prop."
                )
            } else {
                format!(
                    "`{name}` is not a Prop: inline handlers only see Props (carried in the custom_id). Make it a Prop, or move this markup into a sub-View."
                )
            };
            errors.push(syn::Error::new(ident.span(), message));
        }
        errors.finish()
    }

    fn select(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        tag: Tag,
        mut attrs: Attrs,
        ty: TokenStream,
        close: TokenStream,
        refs: TokenStream,
    ) -> syn::Result<TokenStream> {
        let c = cmd();
        let t = types();
        let name = format!("{tag:?}");
        let (custom_id, value_ty, modal) = if self.modal.is_some() {
            if let Some(event) = attrs.event() {
                return err(
                    event.span,
                    "inputs in a modal are read by the `<Modal>`'s `on:submit`",
                );
            }
            let input = self.input_name(&mut attrs, &name)?;
            let ty = self.modal_param(&input)?;
            (quote!(::std::string::String::from(#input)), ty, true)
        } else {
            match (attrs.event(), attrs.value(&["custom_id"])?) {
                (Some(_), Some((span, _))) => {
                    return err(span, "the custom_id comes from `on:select`");
                }
                (Some(event), None) => {
                    let (id, handler) = self.event(event, Event::Select, &name)?;
                    let ty = handler
                        .and_then(|i| self.handlers[i].typed.first().map(|(_, ty)| ty.clone()));
                    (id, ty, false)
                }
                (None, Some((_, value))) => (
                    quote!(#c::IntoAttr::<::std::string::String>::into_attr(#value)),
                    None,
                    false,
                ),
                (None, None) => {
                    return err(
                        el.name().span(),
                        format!("a `<{name}>` needs `on:select` or a `custom_id`"),
                    );
                }
            }
        };
        let e = self.var("e");
        let (variant, options) = match tag {
            Tag::StringSelect => {
                let options = self.options(el, &mut attrs, false)?;
                (
                    quote!(StringSelect),
                    quote!(::core::option::Option::Some(#options)),
                )
            }
            other => {
                if let Some(child) = el.children.iter().find(|node| !is_comment(node)) {
                    return err(child.span(), format!("a `<{name}>` takes no children"));
                }
                let variant = match other {
                    Tag::UserSelect => quote!(UserSelect),
                    Tag::RoleSelect => quote!(RoleSelect),
                    Tag::MentionableSelect => quote!(MentionableSelect),
                    _ => quote!(ChannelSelect),
                };
                (variant, quote!(::core::option::Option::None))
            }
        };
        let typed = value_ty.map(|ty| {
            let (trait_, extra) = if modal {
                let shape = shape_check(
                    &ty,
                    "VALUES",
                    &format!("a `<{name}>` submits selections (a value, Option, or Vec)"),
                );
                (
                    quote!(#c::ModalValue),
                    quote! {
                        #shape
                        #e.required = #c::required::<#ty>();
                    },
                )
            } else {
                (quote!(#c::SelectValue), quote!())
            };
            quote! {
                if <#ty as #trait_>::MANY {
                    let all = #e.options.as_ref().map_or(25, ::std::vec::Vec::len).min(25);
                    #e.max_values = ::core::option::Option::Some(all as i32);
                }
                #extra
            }
        });
        let assign = attrs.assign(&e, &[("min", "min_values"), ("max", "max_values")])?;
        Ok(quote! {{
            #refs
            #close
            #[allow(unused_mut)]
            let mut #e = #ty::new(
                #custom_id,
                #options,
                ::core::option::Option::None,
                ::core::option::Option::None,
                ::core::option::Option::None,
                ::core::option::Option::None,
                ::core::option::Option::None,
            );
            #typed
            #assign
            #t::Component::#variant(#e)
        }})
    }

    /// The options of a select (`group` false) or a radio or checkbox group, a `Vec` expression:
    /// `options=ChoiceEnum`, `<Option>` children, `{..}` of options
    fn options(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        attrs: &mut Attrs,
        group: bool,
    ) -> syn::Result<TokenStream> {
        let c = cmd();
        let t = types();
        let ty = if group {
            quote!(#t::GroupOption)
        } else {
            quote!(#t::SelectOption)
        };
        let mut items = Vec::new();
        let mut dynamic = false;
        if let Some((span, choice)) = attrs.value(&["options"])? {
            let choice: Type = syn::parse2(choice.to_token_stream())
                .map_err(|_| syn::Error::new(span, "`options` is a #[derive(Choice)] enum"))?;
            let option = if group {
                quote!(#t::GroupOption::new(*label, *value))
            } else {
                quote!(#t::SelectOption::new(
                    ::std::string::ToString::to_string(label),
                    ::std::string::ToString::to_string(value),
                    ::core::option::Option::None,
                    ::core::option::Option::None,
                    ::core::option::Option::None,
                ))
            };
            dynamic = true;
            items.push(Item::Many(quote! {
                #c::choice_options::<#choice>().iter().map(|(label, value)| #option)
            }));
        }
        let (max, message) = if group {
            (10, "a group holds up to 10 options")
        } else {
            (
                limits::MAX_SELECT_OPTIONS,
                "a select holds up to 25 options",
            )
        };
        let mut count = 0;
        for node in flatten(&el.children) {
            let option = match node {
                Node::Element(option)
                    if element_name(option).is_ok_and(|path| path.is_ident("Option")) =>
                {
                    count += 1;
                    if count == max + 1 {
                        self.error(syn::Error::new(option.name().span(), message));
                    }
                    self.option(option, group).map(Item::One)
                }
                Node::Block(block) => {
                    dynamic = true;
                    Ok(Item::Many(self.block(block)))
                }
                other => err(
                    other.span(),
                    "options are `<Option value=..>\"Label\"</Option>`, `{..}` of options, or `options=ChoiceEnum`",
                ),
            };
            items.extend(self.recover(option));
        }
        Ok(collect_items(items, dynamic, ty))
    }

    fn option(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        group: bool,
    ) -> syn::Result<TokenStream> {
        let c = cmd();
        let mut attrs = Attrs::new(el, "Option")?;
        let refs = std::mem::take(&mut attrs.refs);
        let ty = tag_type(Tag::Option, el, group);
        let close = close_type(Tag::Option, el, group);
        let value = attrs.required(&["value"], "what the option submits")?;
        let label = match (attrs.value(&["label"])?, self.text(&el.children, "Option")) {
            (Some((span, _)), Some(_)) => {
                return err(span, "the label is either `label` or the option's text");
            }
            (Some((_, label)), None) => {
                quote!(#c::IntoAttr::<::std::string::String>::into_attr(#label))
            }
            (None, Some(text)) => text,
            (None, None) => {
                return err(
                    el.name().span(),
                    "an `<Option>` needs a label: its text or `label`",
                );
            }
        };
        let e = self.var("e");
        let assign = attrs.assign(&e, &[])?;
        let new = if group {
            quote!(#ty::new(#label, #c::IntoAttr::<::std::string::String>::into_attr(#value)))
        } else {
            quote!(#ty::new(
                #label,
                #c::IntoAttr::<::std::string::String>::into_attr(#value),
                ::core::option::Option::None,
                ::core::option::Option::None,
                ::core::option::Option::None,
            ))
        };
        Ok(quote! {{
            #refs
            #close
            #[allow(unused_mut)]
            let mut #e = #new;
            #assign
            #e
        }})
    }

    /// A `<Modal>` root, a `ModalView` expression built on the invocation's `root`; an error in
    /// its attributes is recorded and its inputs still generated
    fn modal(&mut self, el: &NodeElement<rstml::Infallible>, root: &Ident) -> TokenStream {
        let header = self.modal_header(el);
        let ModalHeader {
            custom_id,
            title,
            params,
            refs,
        } = self.recover(header).unwrap_or_else(|| {
            let empty = quote!(::std::string::String::new());
            ModalHeader {
                custom_id: empty.clone(),
                title: empty,
                params: None,
                refs: TokenStream::new(),
            }
        });
        self.modal = Some(ModalScope {
            strict: params.is_some(),
            params: params
                .unwrap_or_default()
                .into_iter()
                .map(|(name, ty)| (name, ty, false))
                .collect(),
        });
        let components = self.level(&el.children, Parent::Modal);
        let scope = self.modal.take().expect("set above");
        for (name, _, _) in scope.params.iter().filter(|(_, _, used)| !used) {
            self.error(syn::Error::new(
                name.span(),
                format!("no input in this modal has `name=\"{name}\"`"),
            ));
        }
        let id = self.var("id");
        let v = self.var("v");
        let data = modal_data(tag_type(Tag::Modal, el, false), quote!(#id), title);
        let close = close_type(Tag::Modal, el, false);
        quote! {
            #refs
            #close
            let #id = #custom_id;
            let #v = #components;
            #root.modal(#data, #v)
        }
    }

    /// A missing or unknown attribute is recorded, so the handler is still hoisted
    fn modal_header(&mut self, el: &NodeElement<rstml::Infallible>) -> syn::Result<ModalHeader> {
        let c = cmd();
        let empty = quote!(::std::string::String::new());
        let mut attrs = Attrs::new(el, "Modal")?;
        let refs = std::mem::take(&mut attrs.refs);
        let title = attrs.required(&["title"], "shown at the top, max 45 characters");
        let title = match self.recover(title) {
            Some(title) => title.to_token_stream(),
            None => empty.clone(),
        };
        let (custom_id, params) = match (attrs.event(), attrs.value(&["custom_id"])?) {
            (Some(_), Some((span, _))) => {
                return err(span, "the custom_id comes from `on:submit`");
            }
            (Some(event), None) => {
                let (id, handler) = self.event(event, Event::Submit, "Modal")?;
                (id, handler.map(|i| self.handlers[i].typed.clone()))
            }
            (None, Some((_, value))) => (
                quote!(#c::IntoAttr::<::std::string::String>::into_attr(#value)),
                None,
            ),
            (None, None) => {
                self.error(syn::Error::new(
                    el.name().span(),
                    "a `<Modal>` needs `on:submit` or a `custom_id`",
                ));
                (empty, None)
            }
        };
        for attr in &attrs.list {
            self.error(syn::Error::new(
                attr.span,
                format!("`<Modal>` has no `{}` attribute", attr.name()),
            ));
        }
        Ok(ModalHeader {
            custom_id,
            title,
            params,
            refs,
        })
    }

    fn label(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        mut attrs: Attrs,
        ty: TokenStream,
        close: TokenStream,
        refs: TokenStream,
    ) -> syn::Result<TokenStream> {
        let c = cmd();
        let t = types();
        let text = attrs.required(&["text", "label"], "shown above the input")?;
        // `Some(None)`: the input had an error, recorded
        let mut input: Option<Option<TokenStream>> = None;
        for node in &el.children {
            match node {
                Node::Element(child) if input.is_some() => {
                    self.error(syn::Error::new(
                        child.name().span(),
                        "a `<Label>` holds one input",
                    ));
                }
                Node::Element(child) => {
                    let tag = element_name(child)
                        .ok()
                        .and_then(|path| path.get_ident().map(Ident::to_string))
                        .and_then(|name| Tag::parse(&name))
                        .filter(|tag| tag.is_input());
                    let component = match tag {
                        Some(tag) => self.component(child, tag),
                        None => err(
                            child.name().span(),
                            "a `<Label>` holds an input: `<TextInput>`, a select, `<RadioGroup>`, `<CheckboxGroup>`, `<Checkbox>` or `<FileUpload>`",
                        ),
                    };
                    input = Some(self.recover(component));
                }
                node if is_comment(node) => {}
                other => self.error(syn::Error::new(other.span(), "a `<Label>` holds one input")),
            }
        }
        let input = match input {
            Some(Some(input)) => input,
            Some(None) => placeholder(),
            None => return err(el.name().span(), "a `<Label>` holds one input"),
        };
        let e = self.var("e");
        let assign = attrs.assign(&e, &[])?;
        Ok(quote! {{
            #refs
            #close
            let #e = #input;
            #[allow(unused_mut)]
            let mut #e = #ty::new(#c::IntoAttr::<::std::string::String>::into_attr(#text), #e);
            #assign
            #t::Component::Label(#e)
        }})
    }

    /// A modal input's `name` (its `custom_id`), a string literal
    fn input_name(&mut self, attrs: &mut Attrs, tag: &str) -> syn::Result<LitStr> {
        match attrs.value(&["name"])? {
            Some((
                _,
                Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(name),
                    ..
                }),
            )) => Ok(name),
            Some((span, _)) => err(span, "`name` is a string literal"),
            None => err(
                Span::call_site(),
                format!(
                    "a `<{tag}>` in a modal needs `name=\"..\"`, matching an `on:submit` parameter"
                ),
            ),
        }
    }

    /// The submit parameter's type for the input named `name`
    fn modal_param(&mut self, name: &LitStr) -> syn::Result<Option<Type>> {
        let scope = self.modal.as_mut().expect("in a modal");
        let value = name.value();
        match scope
            .params
            .iter_mut()
            .find(|(param, _, _)| *param == value)
        {
            Some((_, _, true)) => err(name.span(), format!("two inputs are named `{value}`")),
            Some((_, ty, used)) => {
                *used = true;
                Ok(Some(ty.clone()))
            }
            None if scope.strict => err(
                name.span(),
                format!("the `on:submit` handler has no `{value}: Type` parameter"),
            ),
            None => Ok(None),
        }
    }

    /// A modal input other than a select
    fn input(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        tag: Tag,
        mut attrs: Attrs,
        ty_path: TokenStream,
        close: TokenStream,
        refs: TokenStream,
    ) -> syn::Result<TokenStream> {
        let c = cmd();
        let t = types();
        let name_str = format!("{tag:?}");
        if self.modal.is_none() {
            return err(el.name().span(), tag.placement());
        }
        if let Some(event) = attrs.event() {
            return err(
                event.span,
                "inputs in a modal are read by the `<Modal>`'s `on:submit`",
            );
        }
        let name = self.input_name(&mut attrs, &name_str)?;
        let ty = self.modal_param(&name)?;
        let e = self.var("e");
        let required = |shape: &str, expected: &str| {
            ty.as_ref().map(|ty| {
                let check = shape_check(ty, shape, expected);
                quote! {
                    #check
                    #e.required = #c::required::<#ty>();
                }
            })
        };
        let many = |max: u8| {
            ty.as_ref().map(|ty| {
                quote! {
                    if <#ty as #c::ModalValue>::MANY && #e.max_values.is_none() {
                        #e.max_values = ::core::option::Option::Some(#max);
                    }
                }
            })
        };
        let no_children = || match el.children.iter().find(|node| !is_comment(node)) {
            Some(child) => err(child.span(), format!("a `<{name_str}>` takes no children")),
            None => Ok(()),
        };
        Ok(match tag {
            Tag::TextInput => {
                no_children()?;
                let style = match (attrs.flag("paragraph")?, attrs.value(&["style"])?) {
                    (Some(_), Some((span, _))) => {
                        return err(span, "use `paragraph` or `style`");
                    }
                    (Some(_), None) => quote!(#t::TextInputStyle::Paragraph),
                    (None, Some((_, style))) => {
                        quote!(#c::IntoAttr::<#t::TextInputStyle>::into_attr(#style))
                    }
                    (None, None) => quote!(#t::TextInputStyle::Short),
                };
                let required = required(
                    "TEXT",
                    "a text input submits a single value (String, Option<String>, or a Choice enum)",
                );
                let assign =
                    attrs.assign(&e, &[("min_len", "min_length"), ("max_len", "max_length")])?;
                quote! {{
                    #refs
                    #close
                    let mut #e = #ty_path::new(
                        ::std::string::String::from(#name),
                        #style,
                        ::std::string::String::new(),
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                    );
                    #e.label = ::core::option::Option::None;
                    #required
                    #assign
                    #t::Component::TextInput(#e)
                }}
            }
            Tag::RadioGroup | Tag::CheckboxGroup => {
                let options = self.options(el, &mut attrs, true)?;
                let (variant, required) = if tag == Tag::RadioGroup {
                    (
                        quote!(RadioGroup),
                        required(
                            "TEXT",
                            "a radio group submits a single value (String, Option<String>, or a Choice enum)",
                        ),
                    )
                } else {
                    (
                        quote!(CheckboxGroup),
                        required(
                            "VALUES",
                            "a checkbox group submits selections (a value, Option, or Vec)",
                        ),
                    )
                };
                let assign = attrs.assign(&e, &[("min", "min_values"), ("max", "max_values")])?;
                quote! {{
                    #refs
                    #close
                    #[allow(unused_mut)]
                    let mut #e = #ty_path::new(#name, #options);
                    #required
                    #assign
                    #t::Component::#variant(#e)
                }}
            }
            Tag::Checkbox => {
                no_children()?;
                let check = ty
                    .as_ref()
                    .map(|ty| shape_check(ty, "CHECKED", "a checkbox submits a bool"));
                let assign = attrs.assign(&e, &[])?;
                quote! {{
                    #refs
                    #close
                    #check
                    #[allow(unused_mut)]
                    let mut #e = #ty_path::new(#name);
                    #assign
                    #t::Component::Checkbox(#e)
                }}
            }
            Tag::FileUpload => {
                no_children()?;
                let required = required(
                    "VALUES",
                    "a file upload submits attachments (Attachment, Option, or Vec)",
                );
                let assign = attrs.assign(&e, &[("min", "min_values"), ("max", "max_values")])?;
                let many = many(10);
                quote! {{
                    #refs
                    #close
                    #[allow(unused_mut)]
                    let mut #e = #ty_path::new(#name);
                    #required
                    #assign
                    #many
                    #t::Component::FileUpload(#e)
                }}
            }
            _ => unreachable!("selects are handled by `select`"),
        })
    }

    /// `<Card title=.. likes=3/>`: a user View, built through its Props builder (a value to push
    /// into the level)
    fn user_view(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        path: syn::Path,
    ) -> syn::Result<TokenStream> {
        let c = cmd();
        let attrs = Attrs::new(el, &path.to_token_stream().to_string())?;
        let mut setters = TokenStream::new();
        for attr in attrs.list {
            if attr.key.len() != 1 {
                return err(
                    attr.span,
                    "events go on components; give the View a handler instead",
                );
            }
            let key = &attr.key[0];
            let value = attr.value_or_true();
            setters.extend(quote_spanned!(attr.span=> .#key(#value)));
        }
        let children: Vec<&Node> = el.children.iter().filter(|n| !is_comment(n)).collect();
        if !children.is_empty() {
            // The children are a View of their own, merged into this one when the sub-View is
            // placed
            let root = self.var("root");
            self.roots.push(quote!((&mut #root)));
            let components = self.level(children, Parent::Any);
            self.roots.pop();
            let v = self.var("v");
            setters.extend(quote! {
                .children({
                    #[allow(unused_mut)]
                    let mut #root = #c::MarkupRoot::default();
                    let #v = #components;
                    #root.view(#v)
                })
            });
        }
        let is_self = self
            .scope
            .as_ref()
            .is_some_and(|scope| path.is_ident(&scope.name));
        if !is_self
            && !self.views.iter().any(|view| {
                view.to_token_stream().to_string() == path.to_token_stream().to_string()
            })
        {
            self.views.push(path.clone());
        }
        // A tag name still being typed: complete it from the built-in tags too
        let complete = match path.get_ident() {
            Some(ident) if self.incomplete && tags::completes(&ident.to_string()) => {
                quote!(use ::composure::command::tags::#ident as _;)
            }
            _ => TokenStream::new(),
        };
        Ok(quote! {{
            #complete
            #path::builder() #setters .build()
        }})
    }
}

/// Discord's limits that `rsx!` checks at compile time; the runtime check uses
/// `composure::types::limits`, which these mirror
mod limits {
    pub const MAX_ROW_BUTTONS: usize = 5;
    pub const SECTION_TEXTS_MAX: usize = 3;
    pub const MODAL_COMPONENTS_MAX: usize = 5;
    pub const GALLERY_ITEMS_MAX: usize = 10;
    pub const MAX_SELECT_OPTIONS: usize = 25;
}

/// A `<Modal>`'s attributes
struct ModalHeader {
    /// A `String` expression
    custom_id: TokenStream,
    title: TokenStream,
    /// The inline `on:submit`'s typed parameters
    params: Option<Vec<(Ident, Type)>>,
    /// References to the attributes' docs in `composure::command::tags`
    refs: TokenStream,
}

/// What a level's element is
enum Kind {
    Tag(Tag),
    /// A `#[view]` placed as a sub-View
    View(syn::Path),
}

fn classify(el: &NodeElement<rstml::Infallible>) -> syn::Result<Kind> {
    let path = element_name(el)?;
    if let Some(tag) = path
        .get_ident()
        .and_then(|ident| Tag::parse(&ident.to_string()))
    {
        return Ok(Kind::Tag(tag));
    }
    let last = &path.segments.last().expect("a path").ident;
    if last.to_string().starts_with(char::is_uppercase) {
        Ok(Kind::View(path))
    } else {
        err(el.name().span(), format!("unknown tag `<{last}>`"))
    }
}

/// One generated child of a level
enum Child {
    /// A `Component` expression
    Component(TokenStream),
    /// A value pushed with `IntoChildren`: a `{..}`, a sub-View
    Push(TokenStream),
}

/// A level's nodes: runs of adjacent text (joined into one text display), and the rest
enum Piece<'n> {
    Text(Vec<&'n LitStr>),
    Node(&'n Node),
}

fn pieces<'n>(nodes: &[&'n Node]) -> Vec<Piece<'n>> {
    let mut pieces = Vec::new();
    for node in nodes {
        match (node, pieces.last_mut()) {
            (Node::Text(text), Some(Piece::Text(lits))) => lits.push(&text.value),
            (Node::Text(text), _) => pieces.push(Piece::Text(vec![&text.value])),
            (node, _) => pieces.push(Piece::Node(node)),
        }
    }
    pieces
}

/// `nodes` without comments, with fragments' children in their place
fn flatten<'n>(nodes: impl IntoIterator<Item = &'n Node>) -> Vec<&'n Node> {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            Node::Fragment(fragment) => out.extend(flatten(&fragment.children)),
            node if is_comment(node) => {}
            node => out.push(node),
        }
    }
    out
}

/// Adjacent string literals as one, at the first one's span
fn fold(lits: &[&LitStr]) -> LitStr {
    match lits {
        [lit] => (*lit).clone(),
        _ => LitStr::new(
            &lits.iter().map(|lit| lit.value()).collect::<String>(),
            lits[0].span(),
        ),
    }
}

/// Bytes reserved for each `{..}` in a text-only element, besides its literals
const EXPR_CAPACITY: usize = 16;

/// Part of a text-only element's text
enum TextPart<'n> {
    Lits(Vec<&'n LitStr>),
    Expr(TokenStream),
}

/// An item of a `Vec` markup builds (gallery items, options)
enum Item {
    One(TokenStream),
    /// An iterator of items, extended with
    Many(TokenStream),
}

/// A `Vec<ty>` expression: `vec![..]` when every item is one, else pushed and extended
fn collect_items(items: Vec<Item>, dynamic: bool, ty: TokenStream) -> TokenStream {
    if !dynamic {
        let items = items.into_iter().map(|item| match item {
            Item::One(item) | Item::Many(item) => item,
        });
        return quote!(::std::vec![#(#items),*]);
    }
    let var = Ident::new("__items", Span::mixed_site());
    let statements = items.into_iter().map(|item| match item {
        Item::One(item) => quote!(#var.push(#item);),
        Item::Many(items) => quote!(::core::iter::Extend::extend(&mut #var, #items);),
    });
    quote! {{
        let mut #var: ::std::vec::Vec<#ty> = ::std::vec::Vec::new();
        #(#statements)*
        #var
    }}
}

/// The type a built-in tag builds, `::composure::types::X` at the tag name's span, so hover and
/// go-to-definition on `<X>` reach it
fn tag_type(tag: Tag, el: &NodeElement<rstml::Infallible>, group: bool) -> TokenStream {
    spanned_type(tag, el.name().span(), group)
}

fn spanned_type(tag: Tag, span: Span, group: bool) -> TokenStream {
    let name = if group { "GroupOption" } else { tag.def().ty };
    let ident = Ident::new(name, span);
    quote_spanned!(span=> ::composure::types::#ident)
}

/// `let _: X;` at the closing tag's span, so `</X>` reaches the type too
fn close_type(tag: Tag, el: &NodeElement<rstml::Infallible>, group: bool) -> TokenStream {
    match &el.close_tag {
        Some(close) => {
            let ty = spanned_type(tag, close.name.span(), group);
            quote!(let _: #ty;)
        }
        None => TokenStream::new(),
    }
}

/// The names patterns bind in what `visit` walks: `let`s, closure and `fn` parameters, `for`,
/// `match` and `if let` patterns. Capitalized names are left out: in patterns they are enum
/// variants and constants (`None`), not bindings.
fn bindings(visit: impl FnOnce(&mut Bindings)) -> HashSet<String> {
    let mut bindings = Bindings::default();
    visit(&mut bindings);
    bindings.0
}

#[derive(Default)]
struct Bindings(HashSet<String>);

impl<'ast> Visit<'ast> for Bindings {
    fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
        let name = pat.ident.to_string();
        if !name.starts_with(char::is_uppercase) {
            self.0.insert(name);
        }
        visit::visit_pat_ident(self, pat);
    }
}

/// The plain names `expr` reads as values: single-segment paths (`total`, `Card { title }`), and
/// in macros the identifiers not used as a method, field, path segment or macro name, plus
/// `{name}` in format strings
fn uses(expr: &Expr) -> Vec<Ident> {
    #[derive(Default)]
    struct Uses(Vec<Ident>);

    impl<'ast> Visit<'ast> for Uses {
        fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
            if path.qself.is_none()
                && path.path.leading_colon.is_none()
                && let [segment] = path.path.segments.iter().collect::<Vec<_>>().as_slice()
                && segment.arguments.is_none()
            {
                self.0.push(segment.ident.clone());
            }
            visit::visit_expr_path(self, path);
        }

        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            macro_uses(mac.tokens.clone(), &mut self.0);
        }
    }

    let mut uses = Uses::default();
    uses.visit_expr(expr);
    uses.0
}

fn macro_uses(tokens: TokenStream, out: &mut Vec<Ident>) {
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    let punct = |tree: Option<&TokenTree>, c: char| matches!(tree, Some(TokenTree::Punct(punct)) if punct.as_char() == c);
    for (i, tree) in trees.iter().enumerate() {
        match tree {
            TokenTree::Ident(ident) => {
                let before = i.checked_sub(1).and_then(|i| trees.get(i));
                let after = trees.get(i + 1);
                let member = punct(before, '.') || punct(before, ':');
                let path_or_macro =
                    punct(after, '!') || (punct(after, ':') && punct(trees.get(i + 2), ':'));
                if !member && !path_or_macro {
                    out.push(ident.clone());
                }
            }
            TokenTree::Group(group) => macro_uses(group.stream(), out),
            TokenTree::Literal(literal) => {
                let text = literal.to_string();
                if text.ends_with('"') {
                    let mut names = HashSet::new();
                    format_args(&text, &mut names);
                    out.extend(
                        names
                            .into_iter()
                            .filter(|name| {
                                name.starts_with(|c: char| c.is_alphabetic() || c == '_')
                            })
                            .map(|name| Ident::new(&name, literal.span())),
                    );
                }
            }
            TokenTree::Punct(_) => {}
        }
    }
}

/// A `ModalCallbackData` (`ty`, spanned) with this `custom_id` (a `String`) and title, its
/// components left to `MarkupRoot::modal`
fn modal_data(ty: TokenStream, custom_id: TokenStream, title: TokenStream) -> TokenStream {
    let c = cmd();
    quote! {
        #ty {
            custom_id: #custom_id,
            title: #c::IntoAttr::<::std::string::String>::into_attr(#title),
            components: ::core::option::Option::None,
            tts: ::core::option::Option::None,
            content: ::core::option::Option::None,
            embeds: ::core::option::Option::None,
            allowed_mentions: ::core::option::Option::None,
            flags: ::core::option::Option::None,
        }
    }
}

/// Stands in for a child whose error is recorded (the macro emits a `compile_error!`), so its
/// parent and siblings are still generated
fn placeholder() -> TokenStream {
    let t = types();
    quote!(#t::Component::Separator(#t::Separator::new()))
}

/// A compile-time check that a submit parameter's type reads what its input submits
fn shape_check(ty: &Type, shape: &str, expected: &str) -> TokenStream {
    let c = cmd();
    let shape = Ident::new(shape, Span::call_site());
    let message = format!("`{}`: {expected}", ty.to_token_stream());
    quote_spanned! {ty.span()=>
        const _: () = ::core::assert!(
            <#ty as #c::ModalValue>::SHAPE & #c::shape::#shape != 0,
            #message,
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PropKind {
    Required,
    /// `Option<T>`: `None` when left out
    Optional,
    /// `#[prop(default = ..)]`
    Default,
    /// `children: View`: the tag's children
    Children,
}

struct Prop {
    name: Ident,
    ty: Type,
    kind: PropKind,
    default: Option<Expr>,
}

fn is_option(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.qself.is_none()
        && path.path.segments.last().is_some_and(|s| s.ident == "Option"
            && matches!(s.arguments, PathArguments::AngleBracketed(_))))
}

/// `#[view]`: keeps the function, adds a same-named Props struct (with a builder for markup), and
/// hoists the inline handlers of its `rsx!`s into a `ViewHandler`
pub fn expand_attr(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut prefix: Option<LitStr> = None;
    if !args.is_empty() {
        let parser = syn::meta::parser(|meta| {
            if meta.path.is_ident("prefix") {
                prefix = Some(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(meta.error("expected `prefix = \"...\"`"))
            }
        });
        syn::parse::Parser::parse2(parser, args)?;
    }

    let mut function: ItemFn = syn::parse2(item)?;
    let name = function.sig.ident.clone();
    let vis = function.vis.clone();
    if let Some(token) = function.sig.asyncness {
        return err(
            token.span,
            "Views render synchronously: handlers can be async",
        );
    }
    if !function.sig.generics.params.is_empty() {
        return err(function.sig.generics.span(), "Views cannot be generic");
    }
    let return_ty = match &function.sig.output {
        ReturnType::Type(_, ty) => (**ty).clone(),
        ReturnType::Default => {
            return err(function.sig.span(), "a View returns `View` or `ModalView`");
        }
    };
    let modal = match &return_ty {
        Type::Path(path) if path.path.segments.last().is_some_and(|s| s.ident == "View") => false,
        Type::Path(path)
            if path
                .path
                .segments
                .last()
                .is_some_and(|s| s.ident == "ModalView") =>
        {
            true
        }
        other => return err(other.span(), "a View returns `View` or `ModalView`"),
    };

    let mut props = Vec::new();
    for input in function.sig.inputs.iter_mut() {
        let FnArg::Typed(param) = input else {
            return err(input.span(), "Views cannot take `self`");
        };
        let Pat::Ident(pat) = &*param.pat else {
            return err(param.pat.span(), "Props must be plain names");
        };
        if matches!(*param.ty, Type::Reference(_)) {
            return err(
                param.ty.span(),
                "Props are owned (handlers decode them from the custom_id): use `String`, not `&str`",
            );
        }
        let mut default = None;
        let mut error = None;
        param.attrs.retain(|attr| {
            if !attr.path().is_ident("prop") {
                return true;
            }
            if let Err(e) = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("default") {
                    default = Some(meta.value()?.parse::<Expr>()?);
                    Ok(())
                } else {
                    Err(meta.error("expected `default = ...`"))
                }
            }) {
                error = Some(e);
            }
            false
        });
        if let Some(error) = error {
            return Err(error);
        }
        let kind = if pat.ident == "children" {
            PropKind::Children
        } else if default.is_some() {
            PropKind::Default
        } else if is_option(&param.ty) {
            PropKind::Optional
        } else {
            PropKind::Required
        };
        props.push(Prop {
            name: pat.ident.clone(),
            ty: (*param.ty).clone(),
            kind,
            default,
        });
    }

    let prefix = match prefix {
        Some(prefix) => prefix,
        None => LitStr::new(&snake_case(&name), name.span()),
    };
    let prefix_value = prefix.value();
    if prefix_value.is_empty() || prefix_value.contains([':', '\\']) {
        return err(
            prefix.span(),
            "the prefix must be non-empty, without `:` or `\\`",
        );
    }

    let mut gen_ = Gen::new(Some(Scope {
        name: name.clone(),
        prefix: prefix_value,
        props,
        modal,
    }));
    // Errors in the markup are emitted next to the full expansion (Props, builder, handlers), so
    // code using the View still type-checks
    gen_.locals = bindings(|v| v.visit_block(&function.block));
    let mut expander = Expander { gen_: &mut gen_ };
    expander.visit_block_mut(&mut function.block);
    let scope = gen_.scope.take().expect("set above");

    let c = cmd();
    let t = types();
    let docs: Vec<_> = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .collect();
    let prop_names: Vec<_> = scope.props.iter().map(|p| &p.name).collect();
    let prop_types: Vec<_> = scope.props.iter().map(|p| &p.ty).collect();

    let builder = props_builder(&name, &vis, &scope.props);

    let message_impls = (!modal).then(|| {
        quote! {
            impl #c::IntoChildren for #name {
                fn push_into(self, children: &mut #c::Children<'_>) {
                    #c::IntoChildren::push_into(self.render(), children)
                }
            }

            impl #c::Rerender for #name {
                fn rerender(
                    self,
                ) -> ::core::result::Result<#t::InteractionResponse, #c::HandlerError> {
                    #c::Rerender::rerender(self.render())
                }
            }
        }
    });

    let handled = !gen_.handlers.is_empty();
    let handler = view_handler(&name, &prefix, &gen_.handlers);
    let handler = gen_.recover(handler);
    let views = &gen_.views;
    let route = crate::app::view_route(&name);
    let errors = std::mem::take(&mut gen_.errors).into_compile_error();

    Ok(quote! {
        #errors


        #[allow(non_snake_case)]
        #function

        #(#docs)*
        #[allow(dead_code)]
        #vis struct #name {
            #(pub #prop_names: #prop_types,)*
        }

        #builder

        impl #name {
            /// This View rendered from these Props
            #[allow(dead_code)]
            #vis fn render(self) -> #return_ty {
                #name(#(self.#prop_names),*)
            }
        }

        #message_impls

        impl #c::IntoResponse for #name {
            fn into_response(self) -> #t::InteractionResponse {
                #c::IntoResponse::into_response(self.render())
            }

            fn try_into_response(
                self,
            ) -> ::core::result::Result<#t::InteractionResponse, #c::HandlerError> {
                #c::IntoResponse::try_into_response(self.render())
            }
        }

        impl #c::ViewDefinition for #name {
            const INFO: #c::ViewInfo = #c::ViewInfo {
                prefix: #prefix,
                handled: #handled,
                views: &[#(<#views as #c::ViewDefinition>::INFO),*],
            };
        }

        #handler
        #route
    })
}

/// A typestate builder for markup (`<Card title=.. likes=3/>`): required Props change the type
/// when set, so `build` exists only once all are
fn props_builder(name: &Ident, vis: &syn::Visibility, props: &[Prop]) -> TokenStream {
    let c = cmd();
    let builder = format_ident!("__{}Builder", name);
    let required: Vec<usize> = props
        .iter()
        .enumerate()
        .filter(|(_, p)| p.kind == PropKind::Required)
        .map(|(i, _)| i)
        .collect();
    let param = |i: usize| format_ident!("__P{}", i);
    let missing = |p: &Prop| format_ident!("__{}_missing_{}", name, p.name);

    let fields = props.iter().enumerate().map(|(i, p)| {
        let field = &p.name;
        if p.kind == PropKind::Required {
            let param = param(i);
            quote!(#field: #param)
        } else {
            let ty = &p.ty;
            quote!(#field: #ty)
        }
    });
    let params: Vec<_> = required.iter().map(|&i| param(i)).collect();
    let markers: Vec<_> = required.iter().map(|&i| missing(&props[i])).collect();
    let initial = props.iter().map(|p| {
        let field = &p.name;
        match p.kind {
            PropKind::Required => {
                let marker = missing(p);
                quote!(#field: #marker)
            }
            PropKind::Optional => quote!(#field: ::core::option::Option::None),
            PropKind::Default => {
                let default = p.default.as_ref().expect("a default");
                quote!(#field: #default)
            }
            PropKind::Children => quote!(#field: ::core::default::Default::default()),
        }
    });

    let setters = props.iter().enumerate().map(|(i, p)| {
        let field = &p.name;
        let ty = &p.ty;
        if p.kind != PropKind::Required {
            return quote! {
                impl<#(#params),*> #builder<#(#params),*> {
                    #[allow(dead_code)]
                    #vis fn #field(mut self, value: impl #c::IntoAttr<#ty>) -> Self {
                        self.#field = #c::IntoAttr::into_attr(value);
                        self
                    }
                }
            };
        }
        let others: Vec<_> = required
            .iter()
            .filter(|&&j| j != i)
            .map(|&j| param(j))
            .collect();
        let before: Vec<TokenStream> = required
            .iter()
            .map(|&j| {
                if j == i {
                    missing(&props[j]).to_token_stream()
                } else {
                    param(j).to_token_stream()
                }
            })
            .collect();
        let after: Vec<TokenStream> = required
            .iter()
            .map(|&j| {
                if j == i {
                    ty.to_token_stream()
                } else {
                    param(j).to_token_stream()
                }
            })
            .collect();
        let moves = props.iter().map(|other| {
            let other_field = &other.name;
            if other.name == p.name {
                quote!(#other_field: #c::IntoAttr::into_attr(value))
            } else {
                quote!(#other_field: self.#other_field)
            }
        });
        quote! {
            impl<#(#others),*> #builder<#(#before),*> {
                #[allow(dead_code)]
                #vis fn #field(self, value: impl #c::IntoAttr<#ty>) -> #builder<#(#after),*> {
                    #builder { #(#moves),* }
                }
            }
        }
    });

    let built: Vec<_> = required.iter().map(|&i| &props[i].ty).collect();
    let all: Vec<_> = props.iter().map(|p| &p.name).collect();
    quote! {
        #(
            #[doc(hidden)]
            #[allow(non_camel_case_types, dead_code)]
            #vis struct #markers;
        )*

        #[doc(hidden)]
        #[allow(non_camel_case_types, dead_code)]
        #vis struct #builder<#(#params),*> {
            #(#fields,)*
        }

        impl #name {
            /// Props set one by one, as markup does: `<Card title="Hi" likes=3/>`
            #[allow(dead_code)]
            #vis fn builder() -> #builder<#(#markers),*> {
                #builder { #(#initial),* }
            }
        }

        #(#setters)*

        impl #builder<#(#built),*> {
            #[allow(dead_code)]
            #vis fn build(self) -> #name {
                #name { #(#all: self.#all),* }
            }
        }
    }
}

/// The `ViewHandler` impl running the View's inline handlers
fn view_handler(name: &Ident, prefix: &LitStr, handlers: &[Handler]) -> syn::Result<TokenStream> {
    let c = cmd();
    let t = types();
    let mut concrete: Option<Type> = None;
    let mut bounds: Vec<TypeParamBound> = Vec::new();
    for handler in handlers {
        let Some(ty) = &handler.ctx_ty else { continue };
        let ctx_name = match handler.event {
            Event::Submit => "ModalCtx",
            _ => "ComponentCtx",
        };
        match ctx_state(ty, ctx_name) {
            Some(Some(Type::ImplTrait(bound))) => bounds.extend(bound.bounds.iter().cloned()),
            Some(Some(state)) => {
                if let Some(existing) = &concrete
                    && existing.to_token_stream().to_string() != state.to_token_stream().to_string()
                {
                    return err(state.span(), "the handlers of a View must use one State");
                }
                concrete = Some(state);
            }
            _ => {}
        }
    }
    let (generics, state) = match concrete {
        Some(state) => (quote!(), state.to_token_stream()),
        None if bounds.is_empty() => (quote!(<__S>), quote!(__S)),
        None => (quote!(<__S: #(#bounds)+*>), quote!(__S)),
    };

    let ctx = Ident::new("ctx", Span::mixed_site());
    let fields = Ident::new("fields", Span::mixed_site());
    let arm = |handler: &Handler| {
        let handler_name = &handler.name;
        let full = format!("{}:{handler_name}", prefix.value());
        let names = handler.captured.iter().map(|(name, _)| name);
        let types = handler.captured.iter().map(|(_, ty)| ty);
        let typed = handler.typed.iter().map(|(param, ty)| match handler.event {
            Event::Submit => {
                let input = param.to_string();
                quote!(let #param: #ty = #c::modal_value::<#ty>(#ctx.data(), #input)?;)
            }
            _ => quote!(let #param: #ty = #c::select_value::<#ty>(#ctx.data())?;),
        });
        // The annotation as written, with the State filled in (`ComponentCtx<'_, __S>`)
        let ctx_ty = handler.ctx_ty.as_ref().map(|ty| {
            let mut ty = ty.clone();
            if let Type::Path(path) = &mut ty
                && let Some(segment) = path.path.segments.last_mut()
            {
                segment.arguments = PathArguments::AngleBracketed(syn::parse_quote!(<'_, #state>));
            }
            quote!(: #ty)
        });
        let bind = match &handler.ctx_pat {
            Some(pat) => quote!(let #pat #ctx_ty = #ctx;),
            None => quote!(let _ = #ctx;),
        };
        let body = &handler.body;
        let run = if handler.asyncness {
            quote!(async move { #body }.await)
        } else if let Expr::Async(_) = body {
            quote!((#body).await)
        } else {
            quote!((move || #body)())
        };
        quote! {
            #handler_name => async {
                #[allow(unused_variables)]
                let (#(#names,)*): (#(#types,)*) = #c::decode_state(#full, #fields)?;
                #(#typed)*
                #bind
                let output = #run;
                #[allow(unused_imports)]
                use #c::{ViaOutput as _, ViaRerender as _};
                (&&#c::HandlerOutput::new(output)).answer()
            }
            .await,
        }
    };
    let method = |event: &[Event], method: TokenStream, ctx_ty: TokenStream| {
        let arms: Vec<_> = handlers
            .iter()
            .filter(|h| event.contains(&h.event))
            .map(arm)
            .collect();
        if arms.is_empty() {
            return quote!();
        }
        quote! {
            async fn #method(
                #ctx: #c::#ctx_ty<'_, #state>,
                #fields: ::core::option::Option<&str>,
            ) -> ::core::option::Option<
                ::core::result::Result<#t::InteractionResponse, #c::HandlerError>,
            > {
                let (handler, #fields) = #c::custom_id::split(#fields.unwrap_or_default());
                ::core::option::Option::Some(match handler {
                    #(#arms)*
                    _ => return ::core::option::Option::None,
                })
            }
        }
    };
    let component = method(
        &[Event::Click, Event::Select],
        quote!(component),
        quote!(ComponentCtx),
    );
    let modal = method(&[Event::Submit], quote!(modal), quote!(ModalCtx));
    Ok(quote! {
        impl #generics #c::ViewHandler<#state> for #name where #state: ::core::marker::Sync {
            #component
            #modal
        }
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use proc_macro2::{LineColumn, TokenStream, TokenTree};

    use super::*;

    /// Every `Container` ident of `tokens` written as `types::Container`, by position
    fn type_paths(tokens: TokenStream, name: &str, out: &mut Vec<LineColumn>) {
        let trees: Vec<TokenTree> = tokens.into_iter().collect();
        for (i, tree) in trees.iter().enumerate() {
            match tree {
                TokenTree::Ident(ident) if ident == name && i >= 3 => {
                    if matches!(&trees[i - 3], TokenTree::Ident(types) if types == "types") {
                        out.push(ident.span().start());
                    }
                }
                TokenTree::Group(group) => type_paths(group.stream(), name, out),
                _ => {}
            }
        }
    }

    #[test]
    fn built_in_tags_keep_their_span() {
        let input = TokenStream::from_str("<Container>\"Hi\"</Container> <Row></Row>").unwrap();
        let output = expand_macro(input);
        let mut containers = Vec::new();
        type_paths(output.clone(), "Container", &mut containers);
        // The opening tag's name (column 1) and the closing tag's (column 17, 0-based), both as the type
        assert!(
            containers.contains(&LineColumn { line: 1, column: 1 }),
            "{containers:?}"
        );
        assert!(
            containers.contains(&LineColumn {
                line: 1,
                column: 17
            }),
            "{containers:?}"
        );
        // An alias names the real type, at the alias's span
        let mut rows = Vec::new();
        type_paths(output, "ActionRow", &mut rows);
        assert!(
            rows.contains(&LineColumn {
                line: 1,
                column: 29
            }),
            "{rows:?}"
        );
    }

    #[test]
    fn every_tag_is_in_the_table() {
        let tags = [
            Tag::Container,
            Tag::Section,
            Tag::Text,
            Tag::Thumbnail,
            Tag::Gallery,
            Tag::Item,
            Tag::File,
            Tag::Separator,
            Tag::Row,
            Tag::Button,
            Tag::StringSelect,
            Tag::UserSelect,
            Tag::RoleSelect,
            Tag::MentionableSelect,
            Tag::ChannelSelect,
            Tag::Option,
            Tag::Modal,
            Tag::Label,
            Tag::TextInput,
            Tag::RadioGroup,
            Tag::CheckboxGroup,
            Tag::Checkbox,
            Tag::FileUpload,
        ];
        assert_eq!(tags.len(), tags::TAGS.len());
        for tag in tags {
            let def = tag.def();
            assert_eq!(Tag::parse(def.name), Some(tag));
            for alias in def.aliases {
                assert_eq!(Tag::parse(alias), Some(tag), "{alias}");
            }
        }
    }

    #[test]
    fn literal_text_is_folded() {
        let input = TokenStream::from_str("<Text>\"a\" \"b\"</Text> \"c\" \"d\"").unwrap();
        let output = expand_macro(input).to_string();
        assert!(output.contains("\"ab\""), "{output}");
        assert!(output.contains("\"cd\""), "{output}");
        // A level without `{..}` or sub-Views is built directly
        assert!(!output.contains("Children"), "{output}");
    }
}
