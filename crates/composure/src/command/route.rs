//! Routes: everything an App lists, and the App itself (ADR 0013).

use std::sync::Arc;

use crate::types::{
    ApplicationCommandInteraction, ApplicationCommandType, Event, InteractionResponse,
    MessageComponentInteraction, ModalSubmitInteraction,
};

use super::{
    Command, CommandDefinition, ComponentHandler, Ctx, EventCtx, HandlerError, Invocation,
    ModalHandler, ScopedDefinition, Theme, ViewHandler, ViewInfo, custom_id,
    set::{scope_eq, str_eq},
};

/// What a Route is, as the compile-time checks of the App listing it see it
#[derive(Debug, Clone, Copy)]
pub enum RouteInfo {
    /// A `#[command]` or `#[group]`
    Command {
        /// The Command's name
        name: &'static str,
        /// Chat input, user, or message
        kind: ApplicationCommandType,
        /// Its named Scope; `None` is global
        scope: Option<&'static str>,
    },
    /// A `#[component]` handler
    Component {
        /// The `custom_id` prefix it answers
        prefix: &'static str,
    },
    /// A `#[modal]` handler
    Modal {
        /// The `custom_id` prefix it answers
        prefix: &'static str,
    },
    /// A `#[view]`
    View(ViewInfo),
    /// An `#[app]`
    App {
        /// What it lists
        routes: &'static [RouteInfo],
        /// Whether this App sets `events`
        events: bool,
    },
}

impl RouteInfo {
    /// A `#[command]` or `#[group]`
    pub const fn command<C: CommandDefinition>() -> Self {
        Self::Command {
            name: C::NAME,
            kind: C::KIND,
            scope: C::SCOPE,
        }
    }

    /// Whether this Route (or one an App lists) answers commands and autocomplete; Apps skip
    /// the others at compile time
    pub const fn has_commands(&self) -> bool {
        answers(self, Kind::Commands)
    }

    /// Whether this Route (or one an App lists) answers components
    pub const fn has_components(&self) -> bool {
        answers(self, Kind::Components)
    }

    /// Whether this Route (or one an App lists) answers modal submits
    pub const fn has_modals(&self) -> bool {
        answers(self, Kind::Modals)
    }

    /// Whether this App or one it lists sets `events`
    pub const fn has_events(&self) -> bool {
        match self {
            Self::App { routes, events } => *events || count(routes, &Query::Events) > 0,
            _ => false,
        }
    }
}

/// What an App lists: a Command, Group, Component or Modal Handler, View, or another App.
/// Implemented by `#[command]`, `#[group]`, `#[component]`, `#[modal]`, `#[view]` and `#[app]`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a Command, Group, Component or Modal Handler, View, or App",
    label = "listed in `#[app(handlers(..))]`",
    note = "list items made with `#[command]`, `#[group]`, `#[component]`, `#[modal]`, `#[view]` or `#[app]`"
)]
pub trait RouteDefinition {
    /// What it is, for the App's compile-time checks
    const ROUTE: RouteInfo;

    /// An App's `theme`; `None` for everything else
    const THEME: Option<&'static Theme> = None;

    /// Appends the Command Definitions of this Route (and of the Routes an App lists), for Sync
    fn definitions(_into: &mut Vec<ScopedDefinition>) {}
}

/// Answers the Interactions routed to it, or `None` when they are not its own. Every method
/// defaults to `None`; each kind of Route overrides the ones it answers. `theme` is the one of
/// the closest App listing it.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be routed with State `{S}`",
    label = "listed in an `#[app]` served with State `{S}`"
)]
pub trait Route<S>: RouteDefinition {
    /// Runs the Command with the invoked name and type
    fn dispatch(
        _interaction: &Arc<ApplicationCommandInteraction>,
        _state: &S,
        _theme: Option<&'static Theme>,
    ) -> impl Future<Output = Option<Result<InteractionResponse, HandlerError>>> + Send {
        async { None }
    }

    /// Runs the invoked Command's Autocomplete Handler for the focused Option
    fn dispatch_autocomplete(
        _interaction: &Arc<ApplicationCommandInteraction>,
        _state: &S,
        _theme: Option<&'static Theme>,
    ) -> impl Future<Output = Option<Result<InteractionResponse, HandlerError>>> + Send {
        async { None }
    }

    /// Runs the Component Handler (or View handler) the `custom_id` prefix names
    fn dispatch_component(
        _interaction: &Arc<MessageComponentInteraction>,
        _state: &S,
        _theme: Option<&'static Theme>,
    ) -> impl Future<Output = Option<Result<InteractionResponse, HandlerError>>> + Send {
        async { None }
    }

    /// Runs the Modal Handler (or View handler) the `custom_id` prefix names
    fn dispatch_modal(
        _interaction: &Arc<ModalSubmitInteraction>,
        _state: &S,
        _theme: Option<&'static Theme>,
    ) -> impl Future<Output = Option<Result<InteractionResponse, HandlerError>>> + Send {
        async { None }
    }

    /// Hands a Webhook Event to the App's `events`; ignored by everything else
    fn dispatch_event(
        _ctx: EventCtx<'_, S>,
        _event: Event,
    ) -> impl Future<Output = Result<(), HandlerError>> + Send {
        async { Ok(()) }
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Commands,
    Components,
    Modals,
}

const fn answers(route: &RouteInfo, kind: Kind) -> bool {
    match (route, kind) {
        (RouteInfo::Command { .. }, Kind::Commands)
        | (RouteInfo::Component { .. }, Kind::Components)
        | (RouteInfo::Modal { .. }, Kind::Modals)
        | (RouteInfo::View(_), Kind::Components | Kind::Modals) => true,
        (RouteInfo::App { routes, .. }, _) => {
            let mut i = 0;
            while i < routes.len() {
                if answers(&routes[i], kind) {
                    return true;
                }
                i += 1;
            }
            false
        }
        _ => false,
    }
}

/// What every [`Route`] dispatch method resolves to; spelled out in the impls the macros generate
#[doc(hidden)]
pub type __Routed = Option<Result<InteractionResponse, HandlerError>>;

/// [`Route::dispatch`] of a Command or Group
#[doc(hidden)]
pub async fn __dispatch_command<C: Command<S>, S: Sync>(
    interaction: &Arc<ApplicationCommandInteraction>,
    state: &S,
    theme: Option<&'static Theme>,
) -> Option<Result<InteractionResponse, HandlerError>> {
    let data = &interaction.data;
    if data.name != C::NAME || data.t != C::KIND {
        return None;
    }
    Some(C::invoke(Invocation::new(interaction, state).with_theme(theme)).await)
}

/// [`Route::dispatch_autocomplete`] of a Command or Group
#[doc(hidden)]
pub async fn __dispatch_autocomplete<C: Command<S>, S: Sync>(
    interaction: &Arc<ApplicationCommandInteraction>,
    state: &S,
    theme: Option<&'static Theme>,
) -> Option<Result<InteractionResponse, HandlerError>> {
    let data = &interaction.data;
    if data.name != C::NAME || data.t != C::KIND {
        return None;
    }
    C::autocomplete(Invocation::new(interaction, state).with_theme(theme)).await
}

/// [`Route::dispatch_component`] of a Component Handler
#[doc(hidden)]
pub async fn __dispatch_component<H: ComponentHandler<S>, S: Sync>(
    interaction: &Arc<MessageComponentInteraction>,
    state: &S,
    theme: Option<&'static Theme>,
) -> Option<Result<InteractionResponse, HandlerError>> {
    let (prefix, fields) = custom_id::split(&interaction.data.custom_id);
    if prefix != H::PREFIX {
        return None;
    }
    Some(H::invoke(Ctx::new(interaction, state).with_theme(theme), fields).await)
}

/// [`Route::dispatch_modal`] of a Modal Handler
#[doc(hidden)]
pub async fn __dispatch_modal<H: ModalHandler<S>, S: Sync>(
    interaction: &Arc<ModalSubmitInteraction>,
    state: &S,
    theme: Option<&'static Theme>,
) -> Option<Result<InteractionResponse, HandlerError>> {
    let (prefix, fields) = custom_id::split(&interaction.data.custom_id);
    if prefix != H::PREFIX {
        return None;
    }
    Some(H::invoke(Ctx::new(interaction, state).with_theme(theme), fields).await)
}

/// [`Route::dispatch_component`] of a View
#[doc(hidden)]
pub async fn __dispatch_view_component<V: ViewHandler<S>, S: Sync>(
    interaction: &Arc<MessageComponentInteraction>,
    state: &S,
    theme: Option<&'static Theme>,
) -> Option<Result<InteractionResponse, HandlerError>> {
    let (prefix, fields) = custom_id::split(&interaction.data.custom_id);
    if prefix != V::INFO.prefix {
        return None;
    }
    V::component(Ctx::new(interaction, state).with_theme(theme), fields).await
}

/// [`Route::dispatch_modal`] of a View
#[doc(hidden)]
pub async fn __dispatch_view_modal<V: ViewHandler<S>, S: Sync>(
    interaction: &Arc<ModalSubmitInteraction>,
    state: &S,
    theme: Option<&'static Theme>,
) -> Option<Result<InteractionResponse, HandlerError>> {
    let (prefix, fields) = custom_id::split(&interaction.data.custom_id);
    if prefix != V::INFO.prefix {
        return None;
    }
    V::modal(Ctx::new(interaction, state).with_theme(theme), fields).await
}

/// An App's theme filled into a response one of its Routes gave, before the Apps around it fill
/// in what it left unset
#[doc(hidden)]
pub fn __themed(
    mut result: Option<Result<InteractionResponse, HandlerError>>,
    theme: Option<&'static Theme>,
) -> Option<Result<InteractionResponse, HandlerError>> {
    if let (Some(theme), Some(Ok(response))) = (theme, &mut result) {
        theme.apply(response);
    }
    result
}

/// Compile-time checks for an App and every App it lists: no two Commands share a name and
/// type, no Scope exceeds Discord's per-type limit, no two handlers share a `custom_id` prefix,
/// sub-Views with handlers that listed Views place are listed too, and at most one App sets
/// `events`. Used by `#[app]`.
#[doc(hidden)]
pub const fn check_app(app: &RouteInfo) {
    let root = std::slice::from_ref(app);
    if count(root, &Query::Events) > 1 {
        panic!("more than one App sets `events`: a bot has one events URL");
    }
    check_routes(root, root);
}

const fn check_routes(root: &[RouteInfo], routes: &[RouteInfo]) {
    let mut i = 0;
    while i < routes.len() {
        match &routes[i] {
            RouteInfo::App { routes, .. } => check_routes(root, routes),
            RouteInfo::Command { name, kind, scope } => {
                if count(root, &Query::Command(name, *kind)) > 1 {
                    panic!("two commands have the same name and type");
                }
                let limit = match kind {
                    ApplicationCommandType::ChatInput => 100,
                    ApplicationCommandType::User | ApplicationCommandType::Message => 15,
                };
                if count(root, &Query::Scope(*kind, *scope)) > limit {
                    panic!(
                        "too many commands of one type in one scope: Discord allows 100 chat-input, 15 user, and 15 message commands"
                    );
                }
            }
            RouteInfo::Component { prefix } => {
                if count(root, &Query::Prefix(prefix, Namespace::Component)) > 1 {
                    panic!("two handlers have the same custom_id prefix");
                }
            }
            RouteInfo::Modal { prefix } => {
                if count(root, &Query::Prefix(prefix, Namespace::Modal)) > 1 {
                    panic!("two handlers have the same custom_id prefix");
                }
            }
            RouteInfo::View(view) => {
                if count(root, &Query::Prefix(view.prefix, Namespace::Component)) > 1
                    || count(root, &Query::Prefix(view.prefix, Namespace::Modal)) > 1
                {
                    panic!("two handlers have the same custom_id prefix");
                }
                check_placed(root, view.views, 0);
            }
        }
        i += 1;
    }
}

/// Every View with handlers that a listed View places (at any depth) is listed too
const fn check_placed(root: &[RouteInfo], placed: &[ViewInfo], depth: usize) {
    if depth > 16 {
        return;
    }
    let mut i = 0;
    while i < placed.len() {
        let view = placed[i];
        if view.handled && count(root, &Query::View(view.prefix)) == 0 {
            panic!(
                "a View with handlers placed by a listed View (a `<SubView .. />` tag in its markup) is missing from `#[app(handlers(..))]`"
            );
        }
        check_placed(root, view.views, depth + 1);
        i += 1;
    }
}

#[derive(Clone, Copy)]
enum Namespace {
    Component,
    Modal,
}

/// What [`count`] looks for
enum Query<'a> {
    Command(&'a str, ApplicationCommandType),
    Scope(ApplicationCommandType, Option<&'a str>),
    /// Handlers answering this prefix: Component or Modal Handlers, and Views, which answer both
    Prefix(&'a str, Namespace),
    View(&'a str),
    /// Apps setting `events`
    Events,
}

/// How many Routes in the tree match `query`
const fn count(routes: &[RouteInfo], query: &Query) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < routes.len() {
        let hit = match (&routes[i], query) {
            (RouteInfo::App { routes, events }, _) => {
                n += count(routes, query);
                *events && matches!(query, Query::Events)
            }
            (RouteInfo::Command { name, kind, .. }, Query::Command(other, other_kind)) => {
                *kind as u8 == *other_kind as u8 && str_eq(name, other)
            }
            (RouteInfo::Command { kind, scope, .. }, Query::Scope(other_kind, other_scope)) => {
                *kind as u8 == *other_kind as u8 && scope_eq(*scope, *other_scope)
            }
            (RouteInfo::Component { prefix }, Query::Prefix(other, Namespace::Component))
            | (RouteInfo::Modal { prefix }, Query::Prefix(other, Namespace::Modal)) => {
                str_eq(prefix, other)
            }
            (RouteInfo::View(view), Query::Prefix(other, _) | Query::View(other)) => {
                str_eq(view.prefix, other)
            }
            _ => false,
        };
        if hit {
            n += 1;
        }
        i += 1;
    }
    n
}
