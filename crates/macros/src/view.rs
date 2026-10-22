//! `rsx!` and `#[view]`: JSX markup (parsed with rstml) expanded to builder calls, as Leptos's
//! `view!` does. A tag is its type's `builder()` at the tag's span, each attribute a method call,
//! each child a `.child(..)` whose bound checks where it may go (`composure::command::RowChild`,
//! ...), and `.build()` at the tag's span; so the compiler checks attributes and placement, and
//! the IDE completes and documents them as methods. The macro keeps what only it can see: inline
//! handlers, hoisted into the View's `ViewHandler` (ADR 0011), a modal's inputs against its
//! submit parameters, required attributes, the counts Discord limits, and locals an inline
//! handler cannot use.

use std::collections::HashSet;

use proc_macro2::{Delimiter, Group, Punct, Spacing, Span, TokenStream, TokenTree};
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
    // The errors after the code: their text can name what is being typed, and rust-analyzer
    // maps its completion cursor between expansions by offset
    let v = Ident::new("__view", Span::mixed_site());
    quote! {{
        let #v = #view;
        #errors
        #v
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
    /// Names the `#[view]` body binds, which hoisted inline handlers cannot see
    locals: HashSet<String>,
    /// Names bound in the `{..}` blocks around the markup being expanded
    enclosing: Vec<HashSet<String>>,
}

/// What a level's children go into, which decides the counts checked and how room is reserved
#[derive(Clone, Copy, PartialEq, Eq)]
enum Parent {
    /// The top of a message, or a sub-View's children (a `View`)
    Top,
    Container,
    Section,
    Row,
    Modal,
    Label,
    Gallery,
    /// A select's options (`group` false) or a radio or checkbox group's
    Options {
        group: bool,
    },
    /// `<Text>`: text whose room is reserved
    Text,
    /// A button's or an option's label
    Caption,
}

impl Parent {
    /// Whether its builder has `.reserve(n)` for `n` children
    fn reserves(self) -> bool {
        !matches!(self, Parent::Label | Parent::Text | Parent::Caption)
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

    /// The type it builds (whose `builder()` it calls)
    fn ty(self) -> &'static str {
        match self {
            Tag::Container => "Container",
            Tag::Section => "Section",
            Tag::Text => "TextDisplay",
            Tag::Thumbnail => "Thumbnail",
            Tag::Gallery => "MediaGallery",
            Tag::Item => "MediaGalleryItem",
            Tag::File => "FileComponent",
            Tag::Separator => "Separator",
            Tag::Row => "ActionRow",
            Tag::Button => "ButtonComponent",
            Tag::StringSelect => "StringSelect",
            Tag::UserSelect => "UserSelect",
            Tag::RoleSelect => "RoleSelect",
            Tag::MentionableSelect => "MentionableSelect",
            Tag::ChannelSelect => "ChannelSelect",
            Tag::Option => "SelectOption",
            Tag::Modal => "ModalView",
            Tag::Label => "Label",
            Tag::TextInput => "TextInput",
            Tag::RadioGroup => "RadioGroup",
            Tag::CheckboxGroup => "CheckboxGroup",
            Tag::Checkbox => "Checkbox",
            Tag::FileUpload => "FileUpload",
        }
    }

    /// `::composure::types::X`, only `X` at `span`: the IDE takes one token for the tag, and the
    /// `composure`/`types` segments would make `<Button>` (built as `ButtonComponent`) a module
    fn path(self, span: Span) -> TokenStream {
        let ident = Ident::new(self.ty(), span);
        match self {
            Tag::Modal => quote!(::composure::command::#ident),
            _ => quote!(::composure::types::#ident),
        }
    }

    /// What its children go into; `None` for a tag without children
    fn children(self) -> Option<Parent> {
        Some(match self {
            Tag::Container => Parent::Container,
            Tag::Section => Parent::Section,
            Tag::Row => Parent::Row,
            Tag::Modal => Parent::Modal,
            Tag::Label => Parent::Label,
            Tag::Gallery => Parent::Gallery,
            Tag::StringSelect => Parent::Options { group: false },
            Tag::RadioGroup | Tag::CheckboxGroup => Parent::Options { group: true },
            Tag::Text => Parent::Text,
            Tag::Button | Tag::Option => Parent::Caption,
            _ => return None,
        })
    }

    /// The event it answers
    fn event(self) -> Option<Event> {
        match self {
            Tag::Button => Some(Event::Click),
            Tag::Modal => Some(Event::Submit),
            tag if tag.is_select() => Some(Event::Select),
            _ => None,
        }
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
}

/// One attribute: its `:`-separated key and its value (`None` for a flag)
#[derive(Clone)]
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

/// An element's attributes, in source order
struct Attrs {
    list: Vec<Attr>,
}

impl Attrs {
    /// The element's attributes, or every malformed one's error, combined
    fn new(el: &NodeElement<rstml::Infallible>) -> syn::Result<Self> {
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
        Ok(Self { list })
    }

    fn get(&self, name: &str) -> Option<&Attr> {
        self.list.iter().find(|attr| attr.is(name))
    }

    fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// `on:..`
    fn event(&self) -> Option<&Attr> {
        self.list.iter().find(|attr| attr.key[0] == "on")
    }
}

/// Most errors one invocation reports; the rest are summed up, so one mistake can't bury the
/// user under cascading errors (as Leptos's `view!` caps them)
const MAX_ERRORS: usize = 16;

/// Errors collected while expanding, reported together: each at the offending token's span,
/// duplicates dropped, at most [`MAX_ERRORS`]
#[derive(Default)]
struct Errors(Vec<syn::Error>);

impl Errors {
    fn push(&mut self, error: syn::Error) {
        // A combined error is several messages: keep them apart, to count and de-duplicate
        self.0.extend(error);
    }

    /// The errors as one, de-duplicated and capped
    fn combined(self) -> Option<syn::Error> {
        let mut seen = HashSet::new();
        let mut out: Option<syn::Error> = None;
        let mut suppressed = 0;
        for error in self.0 {
            if !seen.insert((error.to_string(), format!("{:?}", error.span()))) {
                continue;
            }
            if seen.len() > MAX_ERRORS {
                suppressed += 1;
                continue;
            }
            match &mut out {
                Some(out) => out.combine(error),
                None => out = Some(error),
            }
        }
        if suppressed > 0
            && let Some(out) = &mut out
        {
            out.combine(syn::Error::new(
                Span::call_site(),
                format!("... and {suppressed} more errors in this markup"),
            ));
        }
        out
    }

    fn finish(self) -> syn::Result<()> {
        self.combined().map_or(Ok(()), Err)
    }

    fn into_compile_error(self) -> Option<TokenStream> {
        self.combined().map(syn::Error::into_compile_error)
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
            locals: HashSet::new(),
            enclosing: Vec::new(),
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

    /// The markup's nodes, with rstml's diagnostics recorded; whether there were any.
    ///
    /// As Leptos's `view!` does in debug builds: a tag still being typed (`<Button pri`) has no
    /// `>` yet, so rstml drops it and the IDE has nothing to complete its name or attributes
    /// from. The parse is retried with the tag closed (`/>`), and the retry's nodes are kept when
    /// it has strictly fewer errors and loses no nodes. Leptos appends `/>` at the end only; here
    /// it closes every tag left open when the next one starts (`<Row><Butt</Row>`), so a tag
    /// typed inside another completes too. Unlike Leptos, the first parse's errors are still
    /// reported, so markup missing its `/>` never compiles.
    fn parse(&mut self, tokens: TokenStream) -> (Vec<Node>, bool) {
        let parser = || rstml::Parser::new(rstml::ParserConfig::new().recover_block(true));
        let retry = cfg!(debug_assertions)
            .then(|| close_open_tags(tokens.clone()))
            .flatten();
        let (mut nodes, diagnostics) = parser().parse_recoverable(tokens).split_vec();
        if !diagnostics.is_empty()
            && let Some(retry) = retry
        {
            let (recovered, recovered_diagnostics) = parser().parse_recoverable(retry).split_vec();
            if recovered_diagnostics.len() < diagnostics.len() && recovered.len() >= nodes.len() {
                nodes = recovered;
            }
        }
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
        (nodes, incomplete)
    }

    /// One `rsx!` invocation: a `View`, or a `ModalView` for a `<Modal>` root. Never fails: errors
    /// are recorded and what parsed is still expanded (an empty View or `ModalView` when nothing
    /// did), so `{..}` blocks keep type-checking in incomplete markup
    fn view(&mut self, tokens: TokenStream) -> TokenStream {
        let (nodes, incomplete) = self.parse(tokens);
        self.pre.push(Vec::new());
        let c = cmd();
        let meaningful = flatten(&nodes);
        let modal = match meaningful.as_slice() {
            [Node::Element(el)] if element_name(el).is_ok_and(|path| path.is_ident("Modal")) => {
                Some(el)
            }
            _ => None,
        };
        let in_modal_view = self.scope.as_ref().is_some_and(|scope| scope.modal);
        let body = match modal {
            Some(el) => {
                let modal = self.builtin(el, Tag::Modal);
                self.recover(modal)
                    .unwrap_or_else(|| quote!(#c::ModalView::builder().build()))
            }
            // A `#[view] -> ModalView` whose `<Modal>` is still being typed
            None if incomplete && in_modal_view => quote!(#c::ModalView::builder().build()),
            None => {
                let children = self.children(&nodes, Parent::Top);
                quote!(#c::View::builder() #children .build())
            }
        };
        let pre = self.pre.pop().expect("pushed above");
        // Each child is in braces spanned at its tag (see `children`), as in Leptos's `view!`
        quote! {{
            #[allow(unused_braces)]
            {
                #(#pre)*
                #body
            }
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
            return expr.to_token_stream();
        }
        block.to_token_stream()
    }

    /// `.child(..)` for each of `nodes` (adjacent literals joined), after `.reserve(..)` for the
    /// children the markup shows, so a level allocates once. A node with an error is recorded and
    /// skipped.
    fn children<'n>(
        &mut self,
        nodes: impl IntoIterator<Item = &'n Node>,
        parent: Parent,
    ) -> TokenStream {
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
        let mut calls = TokenStream::new();
        let mut count = 0;
        let mut text = 0;
        let mut exprs = 0;
        for piece in &pieces {
            let child = match piece {
                Piece::Text(lits) => {
                    let lit = fold(lits);
                    text += lit.value().len();
                    Some(lit.to_token_stream())
                }
                Piece::Node(node) => {
                    if let Node::Block(_) = node {
                        exprs += 1;
                    }
                    // In braces spanned at the node: a block's span is its braces, so an error
                    // about the child (it cannot go here) points at it, not at the whole macro
                    let span = match node {
                        Node::Element(el) => el.name().span(),
                        node => node.span(),
                    };
                    self.child(node).map(|child| {
                        let mut group = Group::new(Delimiter::Brace, child);
                        group.set_span(span);
                        group.into_token_stream()
                    })
                }
            };
            if let Some(child) = child {
                if !(parent == Parent::Section && is_accessory(piece)) {
                    count += 1;
                }
                calls.extend(quote!(.child(#child)));
            }
        }
        let reserve = match parent {
            // Room for the literals, and a number or a short name per `{..}`
            Parent::Text if exprs > 0 => Some(text + EXPR_CAPACITY * exprs),
            parent if parent.reserves() && count > 0 => Some(count),
            _ => None,
        };
        match reserve {
            Some(reserve) => quote!(.reserve(#reserve) #calls),
            None => calls,
        }
    }

    /// The counts Discord limits that a level known at compile time shows (ADR 0015); the
    /// rest are checked when the View is answered with
    fn check_counts(&mut self, pieces: &[Piece], parent: Parent) {
        let span = |piece: &Piece| match piece {
            Piece::Text(lits) => lits[0].span(),
            Piece::Node(Node::Element(el)) => el.name().span(),
            Piece::Node(node) => node.span(),
        };
        let counted: Vec<&Piece> = pieces
            .iter()
            .filter(|piece| !(parent == Parent::Section && is_accessory(piece)))
            .collect();
        let (max, message) = match parent {
            Parent::Row => {
                let select = counted.iter().any(|piece| {
                    matches!(piece, Piece::Node(Node::Element(el))
                        if matches!(classify(el), Ok(Kind::Tag(tag)) if tag.is_select()))
                });
                if select && let Some(second) = counted.get(1) {
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
            Parent::Gallery => (
                limits::GALLERY_ITEMS_MAX,
                "a `<Gallery>` holds up to 10 items",
            ),
            Parent::Options { group: false } => (
                limits::MAX_SELECT_OPTIONS,
                "a select holds up to 25 options",
            ),
            Parent::Options { group: true } => {
                (limits::MAX_GROUP_OPTIONS, "a group holds up to 10 options")
            }
            _ => return,
        };
        if let Some(extra) = counted.get(max) {
            self.error(syn::Error::new(span(extra), message));
        }
    }

    /// One node of a level, as the expression its parent's `.child(..)` takes; `None` for
    /// nothing (or an error, recorded)
    fn child(&mut self, node: &Node) -> Option<TokenStream> {
        let result = match node {
            Node::RawText(raw) if !raw.to_token_stream().is_empty() => {
                err(raw.span(), "put text in quotes: \"text\"")
            }
            Node::Block(block) => Ok(Some(self.block(block))),
            Node::Element(el) => match classify(el) {
                Ok(Kind::Tag(tag)) => self.builtin(el, tag).map(Some),
                Ok(Kind::View(path)) => self.user_view(el, path).map(Some),
                Err(error) => Err(error),
            },
            Node::Doctype(doctype) => err(doctype.span(), "unexpected doctype"),
            _ => Ok(None),
        };
        self.recover(result).flatten()
    }

    /// A built-in tag: `Type::builder()`, its attributes as method calls, its children, then
    /// `.build()`; where it may go is the parent's `.child(..)` bound
    fn builtin(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        tag: Tag,
    ) -> syn::Result<TokenStream> {
        let attrs = Attrs::new(el)?;
        // Recorded, and the tag still generated: a tag being typed (no handler yet) keeps its
        // attributes completing and its `{..}` type-checking; the builders have defaults
        for problem in self.check(el, tag, &attrs).0 {
            self.error(problem);
        }
        let span = el.name().span();
        let ty = tag.path(span);
        let (calls, handler) = self.attributes(tag, &attrs)?;
        let mut statements = TokenStream::new();
        let mut extra = TokenStream::new();
        if self.modal.is_some() && tag.is_input() {
            let (calls, checks) = self.modal_input(tag, &attrs, span)?;
            extra.extend(calls);
            statements.extend(checks);
        } else if tag.is_select()
            && let Some(ty) =
                handler.and_then(|i| self.handlers[i].typed.first().map(|(_, ty)| ty.clone()))
        {
            let c = cmd();
            extra.extend(quote!(.many(<#ty as #c::SelectValue>::MANY)));
        }
        let children = match tag.children() {
            Some(parent) => {
                if tag == Tag::Modal {
                    self.modal_children(el, handler)
                } else {
                    self.children(&el.children, parent)
                }
            }
            None => {
                if let Some(child) = el.children.iter().find(|node| !is_comment(node)) {
                    return err(child.span(), format!("`<{tag:?}>` takes no children"));
                }
                TokenStream::new()
            }
        };
        if let Some(close) = &el.close_tag {
            let ty = tag.path(close.name.span());
            statements.extend(quote!(let _: #ty;));
        }
        let build = quote_spanned!(span=> .build());
        let chain = quote!(#ty::builder() #calls #extra #children #build);
        Ok(if statements.is_empty() {
            chain
        } else {
            quote!({ #statements #chain })
        })
    }

    /// A `<Modal>`'s children, its inputs matched to the `on:submit` handler's parameters
    fn modal_children(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        handler: Option<usize>,
    ) -> TokenStream {
        let params = handler.map(|i| self.handlers[i].typed.clone());
        self.modal = Some(ModalScope {
            strict: params.is_some(),
            params: params
                .unwrap_or_default()
                .into_iter()
                .map(|(name, ty)| (name, ty, false))
                .collect(),
        });
        let children = self.children(&el.children, Parent::Modal);
        let scope = self.modal.take().expect("set above");
        for (name, _, _) in scope.params.iter().filter(|(_, _, used)| !used) {
            self.error(syn::Error::new(
                name.span(),
                format!("no input in this modal has `name=\"{name}\"`"),
            ));
        }
        children
    }

    /// What the macro checks of a built-in tag, besides placement and attribute names (the
    /// compiler's): required attributes and conflicting ones, each at the offending token
    fn check(&self, el: &NodeElement<rstml::Infallible>, tag: Tag, attrs: &Attrs) -> Errors {
        let mut errors = Errors::default();
        let mut fail = |span: Span, message: &str| errors.push(syn::Error::new(span, message));
        let span = el.name().span();
        let text_children = || {
            el.children
                .iter()
                .any(|node| matches!(node, Node::Text(_) | Node::Block(_)))
        };
        let elements = || {
            el.children
                .iter()
                .filter_map(|node| match node {
                    Node::Element(child) => Some(child),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let in_modal = self.modal.is_some();
        if let Some(event) = attrs.event() {
            if in_modal && tag.is_input() {
                fail(
                    event.span,
                    "inputs in a modal are read by the `<Modal>`'s `on:submit`",
                );
            } else if tag.event().is_none() {
                fail(event.span, &format!("`<{tag:?}>` has no events"));
            }
        }
        match tag {
            Tag::Button => {
                let link = attrs.has("url") || attrs.has("sku_id");
                let id = attrs.event().or(attrs.get("custom_id"));
                if link && let Some(id) = id {
                    fail(
                        id.span,
                        "a link or premium button cannot have `on:click` or a `custom_id`",
                    );
                } else if !link && id.is_none() {
                    fail(
                        span,
                        "a `<Button>` needs `on:click`, a `custom_id`, or a `url`",
                    );
                }
                if attrs.event().is_some()
                    && let Some(custom_id) = attrs.get("custom_id")
                {
                    fail(custom_id.span, "the custom_id comes from `on:click`");
                }
                let styles: Vec<&Attr> = ["primary", "secondary", "success", "danger", "style"]
                    .iter()
                    .filter_map(|style| attrs.get(style))
                    .collect();
                if let Some(second) = styles.get(1) {
                    fail(second.span, "a button has one style");
                }
                if let Some(label) = attrs.get("label")
                    && text_children()
                {
                    fail(
                        label.span,
                        "the label is either `label` or the button's text",
                    );
                }
            }
            Tag::Section => {
                let accessories: Vec<_> = elements()
                    .into_iter()
                    .filter(|child| {
                        element_name(child)
                            .is_ok_and(|path| path.is_ident("Button") || path.is_ident("Thumbnail"))
                    })
                    .collect();
                if let Some(second) = accessories.get(1) {
                    fail(
                        second.name().span(),
                        "a `<Section>` has one accessory: a `<Button>` or `<Thumbnail>`",
                    );
                }
                match (accessories.is_empty(), attrs.get("accessory")) {
                    (false, Some(attr)) => fail(
                        attr.span,
                        "the accessory is already a `<Button>` or `<Thumbnail>` child",
                    ),
                    (true, None) => fail(
                        span,
                        "a `<Section>` needs an accessory: a `<Button>` or `<Thumbnail>` child",
                    ),
                    _ => {}
                }
            }
            Tag::Text => {
                if let Some(content) = attrs.get("content")
                    && text_children()
                {
                    fail(content.span, "the text is either `content` or the children");
                }
            }
            Tag::Thumbnail | Tag::File | Tag::Item => match (attrs.get("url"), attrs.get("src")) {
                (None, None) => fail(
                    span,
                    &format!("`<{tag:?}>` needs `src` (a URL, or a `File` to upload)"),
                ),
                (Some(_), Some(src)) => fail(src.span, "`src` and `url` are the same: use one"),
                _ => {}
            },
            Tag::Separator => {
                if attrs.has("small")
                    && let Some(large) = attrs.get("large")
                {
                    fail(large.span, "a separator is `small` or `large`");
                }
            }
            Tag::TextInput => {
                if attrs.has("paragraph")
                    && let Some(style) = attrs.get("style")
                {
                    fail(style.span, "use `paragraph` or `style`");
                }
            }
            Tag::Option => {
                if !attrs.has("value") {
                    fail(
                        span,
                        "an `<Option>` needs `value` (what the option submits)",
                    );
                }
                match (attrs.get("label"), text_children()) {
                    (Some(label), true) => fail(
                        label.span,
                        "the label is either `label` or the option's text",
                    ),
                    (None, false) => fail(span, "an `<Option>` needs a label: its text or `label`"),
                    _ => {}
                }
            }
            Tag::Label => {
                if !attrs.has("text") && !attrs.has("label") {
                    fail(span, "a `<Label>` needs `text` (shown above the input)");
                }
                let inputs = elements();
                match inputs.as_slice() {
                    [] => fail(span, "a `<Label>` holds one input"),
                    [_, second, ..] => fail(second.name().span(), "a `<Label>` holds one input"),
                    _ => {}
                }
            }
            Tag::Modal => {
                if !attrs.has("title") {
                    fail(
                        span,
                        "a `<Modal>` needs `title` (shown at the top, max 45 characters)",
                    );
                }
                match (attrs.event(), attrs.get("custom_id")) {
                    (Some(_), Some(custom_id)) => {
                        fail(custom_id.span, "the custom_id comes from `on:submit`")
                    }
                    (None, None) => fail(span, "a `<Modal>` needs `on:submit` or a `custom_id`"),
                    _ => {}
                }
            }
            tag if tag.is_select() && !in_modal => match (attrs.event(), attrs.get("custom_id")) {
                (Some(_), Some(custom_id)) => {
                    fail(custom_id.span, "the custom_id comes from `on:select`")
                }
                (None, None) => fail(
                    span,
                    &format!("a `<{tag:?}>` needs `on:select` or a `custom_id`"),
                ),
                _ => {}
            },
            _ => {}
        }
        errors
    }

    /// The attributes as method calls in source order: `name=value` is `.name(value)` (a flag
    /// `.name(true)`), `on:click=..` `.on_click(custom_id)`, `options=Choice` `.options::<Choice>()`.
    /// Returns the inline handler's index, if one was hoisted.
    fn attributes(&mut self, tag: Tag, attrs: &Attrs) -> syn::Result<(TokenStream, Option<usize>)> {
        let mut calls = TokenStream::new();
        let mut handler = None;
        let mut errors = Errors::default();
        for attr in &attrs.list {
            if attr.key[0] == "on" {
                match self.event(attr.clone(), tag) {
                    Ok((custom_id, index)) => {
                        let method = format_ident!("on_{}", attr.key[1], span = attr.key[1].span());
                        calls.extend(quote!(.#method(#custom_id)));
                        handler = index;
                    }
                    Err(error) => errors.push(error),
                }
                continue;
            }
            if attr.key.len() != 1 {
                errors.push(syn::Error::new(
                    attr.span,
                    format!("`<{tag:?}>` has no `{}` attribute", attr.name()),
                ));
                continue;
            }
            let key = &attr.key[0];
            if key == "options"
                && matches!(
                    tag,
                    Tag::StringSelect | Tag::RadioGroup | Tag::CheckboxGroup
                )
            {
                let choice = attr
                    .value
                    .as_ref()
                    .and_then(|value| syn::parse2::<Type>(value.to_token_stream()).ok());
                match choice {
                    Some(choice) => calls.extend(quote!(.#key::<#choice>())),
                    None => errors.push(syn::Error::new(
                        attr.span,
                        "`options` is a #[derive(Choice)] enum",
                    )),
                }
                continue;
            }
            let value = attr.value_or_true();
            calls.extend(quote!(.#key(#value)));
        }
        errors.finish()?;
        Ok((calls, handler))
    }

    /// The `custom_id` (an `IntoCustomId` expression) for an `on:<event>` attribute: a reference
    /// to a handler (`on:click=like(likes)`), or an inline handler (`on:click:like=|ctx| ..`),
    /// hoisted. Returns the inline handler's index.
    fn event(&mut self, attr: Attr, tag: Tag) -> syn::Result<(TokenStream, Option<usize>)> {
        let Some(expected) = tag.event() else {
            return err(attr.span, format!("`<{tag:?}>` has no events"));
        };
        let event = attr.key.get(1).map(Ident::to_string);
        if event.as_deref() != Some(expected.name()) {
            return err(
                attr.span,
                format!("`<{tag:?}>` answers `on:{}`", expected.name()),
            );
        }
        let key_name = attr.name();
        let Some(value) = attr.value else {
            return err(attr.span, format!("`{key_name}` needs a handler"));
        };
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
                Ok((custom_id, None))
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
                Ok((quote!(#var), Some(self.handlers.len() - 1)))
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

    /// A modal input's checks against the `on:submit` parameter its `name` matches: calls setting
    /// `required` (and `max` for several values) from the parameter's type, and compile-time
    /// checks that the type reads what the input submits
    fn modal_input(
        &mut self,
        tag: Tag,
        attrs: &Attrs,
        span: Span,
    ) -> syn::Result<(TokenStream, TokenStream)> {
        let c = cmd();
        let name = self.input_name(attrs, tag, span)?;
        let Some(ty) = self.modal_param(&name)? else {
            return Ok((TokenStream::new(), TokenStream::new()));
        };
        let (shape, expected) = match tag {
            Tag::TextInput => (
                "TEXT",
                "a text input submits a single value (String, Option<String>, or a Choice enum)"
                    .to_string(),
            ),
            Tag::RadioGroup => (
                "TEXT",
                "a radio group submits a single value (String, Option<String>, or a Choice enum)"
                    .to_string(),
            ),
            Tag::CheckboxGroup => (
                "VALUES",
                "a checkbox group submits selections (a value, Option, or Vec)".to_string(),
            ),
            Tag::Checkbox => ("CHECKED", "a checkbox submits a bool".to_string()),
            Tag::FileUpload => (
                "VALUES",
                "a file upload submits attachments (Attachment, Option, or Vec)".to_string(),
            ),
            _ => (
                "VALUES",
                format!("a `<{tag:?}>` submits selections (a value, Option, or Vec)"),
            ),
        };
        let check = shape_check(&ty, shape, &expected);
        let mut calls = TokenStream::new();
        if tag != Tag::Checkbox {
            calls.extend(quote!(.required_default(#c::required::<#ty>())));
        }
        if tag.is_select() || tag == Tag::FileUpload {
            calls.extend(quote!(.many(<#ty as #c::ModalValue>::MANY)));
        }
        Ok((calls, check))
    }

    /// A modal input's `name` (its `custom_id`), a string literal
    fn input_name(&mut self, attrs: &Attrs, tag: Tag, span: Span) -> syn::Result<LitStr> {
        match attrs.get("name") {
            Some(Attr {
                value:
                    Some(Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(name),
                        ..
                    })),
                ..
            }) => Ok(name.clone()),
            Some(attr) => err(attr.span, "`name` is a string literal"),
            None => err(
                span,
                format!(
                    "a `<{tag:?}>` in a modal needs `name=\"..\"`, matching an `on:submit` parameter"
                ),
            ),
        }
    }

    /// `<Card title=.. likes=3/>`: a user View, built through its Props builder (`.build()` at the
    /// tag, so a missing Prop's error points at it). A single name resolves through the built-in
    /// tags too, so one being typed completes from both.
    fn user_view(
        &mut self,
        el: &NodeElement<rstml::Infallible>,
        path: syn::Path,
    ) -> syn::Result<TokenStream> {
        let c = cmd();
        let attrs = Attrs::new(el)?;
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
            setters.extend(quote!(.#key(#value)));
        }
        let children: Vec<&Node> = el.children.iter().filter(|n| !is_comment(n)).collect();
        if !children.is_empty() {
            let children = self.children(children, Parent::Top);
            setters.extend(quote! {
                .children(#c::View::builder() #children .build())
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
        let builder = match path.get_ident() {
            Some(ident) => {
                let mut block = Group::new(
                    Delimiter::Brace,
                    quote! {
                        #[allow(unused_imports)]
                        use #c::elements::*;
                        #ident::builder()
                    },
                );
                block.set_span(el.name().span());
                block.into_token_stream()
            }
            None => quote!(#path::builder()),
        };
        let build = quote_spanned!(el.name().span()=> .build());
        Ok(quote!(#builder #setters #build))
    }
}

/// `tokens` with `/>` closing each opening tag that the next tag starts before its `>` (or the
/// markup ends), as while it is being typed; `None` when every tag is closed. Attribute values
/// using `<` are in braces, so a bare `<` in an opening tag starts the next tag.
fn close_open_tags(tokens: TokenStream) -> Option<TokenStream> {
    let close = || {
        [
            TokenTree::Punct(Punct::new('/', Spacing::Joint)),
            TokenTree::Punct(Punct::new('>', Spacing::Alone)),
        ]
    };
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    let mut out = Vec::with_capacity(trees.len());
    let mut open = false;
    let mut closed_any = false;
    for (i, tree) in trees.iter().enumerate() {
        if let TokenTree::Punct(punct) = tree {
            match punct.as_char() {
                '<' => {
                    if open {
                        out.extend(close());
                        closed_any = true;
                    }
                    open = matches!(trees.get(i + 1), Some(TokenTree::Ident(_)));
                }
                '>' => open = false,
                _ => {}
            }
        }
        out.push(tree.clone());
    }
    if open {
        out.extend(close());
        closed_any = true;
    }
    closed_any.then(|| out.into_iter().collect())
}

/// Whether a level's piece is a `<Section>`'s accessory
fn is_accessory(piece: &Piece) -> bool {
    matches!(piece, Piece::Node(Node::Element(el))
        if element_name(el).is_ok_and(|path| path.is_ident("Button") || path.is_ident("Thumbnail")))
}

/// Discord's limits that `rsx!` checks at compile time; the runtime check uses
/// `composure::types::limits`, which these mirror
mod limits {
    pub const MAX_ROW_BUTTONS: usize = 5;
    pub const SECTION_TEXTS_MAX: usize = 3;
    pub const MODAL_COMPONENTS_MAX: usize = 5;
    pub const GALLERY_ITEMS_MAX: usize = 10;
    pub const MAX_SELECT_OPTIONS: usize = 25;
    pub const MAX_GROUP_OPTIONS: usize = 10;
}

/// Bytes reserved for each `{..}` in a `<Text>`, besides its literals
const EXPR_CAPACITY: usize = 16;

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
            impl #c::IntoView for #name {
                fn into_view(self) -> #c::View {
                    self.render()
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

        #errors
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

    fn tokens(source: &str) -> TokenStream {
        TokenStream::from_str(source).expect("test markup tokenizes")
    }

    /// What `rsx!` reports for `markup`: each message and where it starts
    fn errors(markup: &str) -> Vec<(String, LineColumn)> {
        let mut gen_ = Gen::new(None);
        let _ = gen_.view(tokens(markup));
        gen_.errors
            .0
            .into_iter()
            .map(|error| (error.to_string(), error.span().start()))
            .collect()
    }

    /// Asserts `markup` reports `message` at the first `at` in it (0-based column, line 1), not at
    /// the macro's call site
    fn assert_error_at(markup: &str, message: &str, at: &str) {
        let column = markup.find(at).expect("`at` is in the markup");
        let found = errors(markup);
        assert!(
            found
                .iter()
                .any(|(text, start)| text.contains(message)
                    && *start == LineColumn { line: 1, column }),
            "`{message}` at column {column}: {found:?}"
        );
    }

    /// The expansion, formatted, for snapshots
    fn expand(markup: &str) -> String {
        let view = expand_macro(tokens(markup));
        let file: syn::File = syn::parse2(quote!(fn view() -> View { #view })).expect("parses");
        prettyplease::unparse(&file)
    }

    /// Every `name` ident written as `types::name` (or `command::name`), by position
    fn type_paths(tokens: TokenStream, name: &str, out: &mut Vec<LineColumn>) {
        let trees: Vec<TokenTree> = tokens.into_iter().collect();
        for (i, tree) in trees.iter().enumerate() {
            match tree {
                TokenTree::Ident(ident) if ident == name && i >= 3 => {
                    if matches!(&trees[i - 3], TokenTree::Ident(m) if m == "types" || m == "command")
                    {
                        out.push(ident.span().start());
                    }
                }
                TokenTree::Group(group) => type_paths(group.stream(), name, out),
                _ => {}
            }
        }
    }

    #[test]
    fn built_in_tags_are_their_type_at_the_tag() {
        let output = expand_macro(tokens("<Container>\"Hi\"</Container> <Row></Row>"));
        let mut containers = Vec::new();
        type_paths(output.clone(), "Container", &mut containers);
        // The opening tag's name (column 1) and the closing tag's (column 17, 0-based)
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
    fn every_tag_names_its_type() {
        for name in [
            "Container",
            "Section",
            "Text",
            "TextDisplay",
            "Thumbnail",
            "Gallery",
            "MediaGallery",
            "Item",
            "MediaGalleryItem",
            "File",
            "Separator",
            "Row",
            "ActionRow",
            "Button",
            "StringSelect",
            "Select",
            "UserSelect",
            "RoleSelect",
            "MentionableSelect",
            "ChannelSelect",
            "Option",
            "Modal",
            "Label",
            "TextInput",
            "RadioGroup",
            "CheckboxGroup",
            "Checkbox",
            "FileUpload",
        ] {
            let tag = Tag::parse(name).unwrap_or_else(|| panic!("`{name}` is a tag"));
            assert!(!tag.ty().is_empty());
        }
    }

    #[test]
    fn literals_are_joined_and_room_reserved() {
        let output = expand_macro(tokens("<Text>\"a\" \"b\" {x}</Text> \"c\" \"d\"")).to_string();
        assert!(output.contains("\"ab\""), "{output}");
        assert!(output.contains("\"cd\""), "{output}");
        // 2 bytes of literals and room for one `{..}`
        assert!(output.contains("reserve (18usize)"), "{output}");
    }

    #[test]
    fn errors_point_at_the_offending_token() {
        assert_error_at(
            "<Section><Text>\"a\"</Text></Section>",
            "needs an accessory",
            "Section",
        );
        assert_error_at(
            "<Row><Button>\"x\"</Button></Row>",
            "needs `on:click`",
            "Button",
        );
        assert_error_at(
            "<Modal custom_id=\"m\"><Label text=\"a\"><TextInput name=\"a\"/></Label></Modal>",
            "needs `title`",
            "Modal",
        );
        assert_error_at("<Container id=1 id=2/>", "duplicate attribute `id`", "id=2");
        assert_error_at("<Text/><foo/>", "unknown tag `<foo>`", "foo");
        assert_error_at(
            "<Row><Button on:select=pick custom_id=\"x\"/></Row>",
            "answers `on:click`",
            "on:",
        );
    }

    #[test]
    fn errors_are_deduplicated_and_capped() {
        let mut errors = Errors::default();
        let span = Span::call_site();
        for _ in 0..3 {
            errors.push(syn::Error::new(span, "same"));
        }
        assert_eq!(errors.combined().into_iter().flatten().count(), 1);

        let mut errors = Errors::default();
        for i in 0..MAX_ERRORS + 4 {
            errors.push(syn::Error::new(span, format!("error {i}")));
        }
        let messages: Vec<String> = errors
            .combined()
            .into_iter()
            .flatten()
            .map(|error| error.to_string())
            .collect();
        assert_eq!(messages.len(), MAX_ERRORS + 1);
        assert_eq!(messages[MAX_ERRORS], "... and 4 more errors in this markup");
    }

    #[test]
    fn tags_being_typed_are_closed_for_the_retry() {
        let closed =
            |markup: &str| close_open_tags(tokens(markup)).map(|t| t.to_string().replace(' ', ""));
        assert_eq!(
            closed("<Row><Butt</Row>").as_deref(),
            Some("<Row><Butt/></Row>")
        );
        assert_eq!(
            closed("<Row><Button pri").as_deref(),
            Some("<Row><Buttonpri/>")
        );
        assert_eq!(closed("<Row><Button/></Row>"), None);
        // A half-typed tag is still generated, so its attributes complete
        let output = expand_macro(tokens("<Row><Button pri</Row>")).to_string();
        assert!(output.contains(". pri (true)"), "{output}");
        // And the markup still fails to compile
        assert!(!errors("<Row><Button pri</Row>").is_empty());
    }

    /// What rust-analyzer expands when completing at `$`: its marker identifier in place of the
    /// cursor, in markup that often does not parse
    fn completing(markup: &str) -> String {
        let input = markup.replace('$', "raCompletionMarker");
        expand_macro(tokens(&input)).to_string().replace(' ', "")
    }

    /// rust-analyzer maps the cursor between the expansions with and without its marker by
    /// offset, so they may differ only by the marker
    #[test]
    fn the_completion_marker_changes_nothing_else() {
        for markup in [
            "<Con$",
            "<Row><Butt$</Row>",
            "<Row><Button pri$ custom_id=\"x\">\"a\"</Button></Row>",
            "<Row><Button pri$</Row>",
            "<Container acc$>\"a\"</Container>",
            "<Text>{x$}</Text> <Con",
            "<Unknown$ a=1/>",
        ] {
            let typed = expand_macro(tokens(&markup.replace('$', "")))
                .to_string()
                .replace(' ', "");
            let completing = completing(markup).replace("raCompletionMarker", "");
            assert_eq!(typed, completing, "{markup}");
        }
    }

    #[test]
    fn snapshot_static_card() {
        insta::assert_snapshot!(expand(
            r###"<Container accent_color=0x5865F2>
                "## " "Components V2"
                <Section>
                    <Text>"Beside a thumbnail"</Text>
                    <Thumbnail src="https://example.com/a.png"/>
                </Section>
                <Separator/>
                <Row><Button primary custom_id="like">"Like"</Button></Row>
            </Container>"###
        ));
    }

    #[test]
    fn snapshot_dynamic_level() {
        insta::assert_snapshot!(expand(
            r###"<Container>
                "## " {title}
                {items.iter().map(|item| rsx! { <Text>"- " {*item}</Text> })}
                <Card title="nested"><Text>"child"</Text></Card>
                <Row><StringSelect on:select=pick(id) options=Color/></Row>
            </Container>"###
        ));
    }

    #[test]
    fn snapshot_modal() {
        insta::assert_snapshot!(expand(
            r###"<Modal title="Feedback" custom_id="feedback">
                <Label text="Topic"><TextInput name="topic" max_len=100/></Label>
                <Label text="Kind"><RadioGroup name="kind" options=Kind/></Label>
            </Modal>"###
        ));
    }
}
