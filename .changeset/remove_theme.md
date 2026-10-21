---
composure: major
composure_macros: major
---

# The Theme is removed; `#[app(errors = ..)]` renders handler errors

The Theme filled in embed colors and branding and, for Components V2, only top-level Container accents, and never reached REST messages, so bots set their colors themselves anyway (ADR 0015). Removed: `command::Theme`, `#[app(theme = ..)]`, `RouteDefinition::THEME`, `InteractionHandler::theme`, `ctx.theme()` / `DeferredCtx::theme()`, `Unthemed`, `MessageCallbackData::unthemed` and `HandlerError::response_with`. Keep your colors as `const`s and set them where you build messages, for example with a wrapper View per tone (see "Styling" in the Views chapter).

Handler errors are rendered by a hook instead: `#[app(.., errors = render)]` with `fn render(error: &HandlerError) -> InteractionResponse` (`command::ErrorRenderer`), used for the initial response and when deferred work fails. At most one App in a tree sets it; the root uses it for the whole tree. Adapters call the new `InteractionHandler::render_error`; without a hook, errors are the ephemeral text of `HandlerError::response`. `HandlerError::user_message()` is now public, for renderers. `Ctx::with_theme` / `Invocation::with_theme` become `with_errors`, and the `Route::dispatch*` methods take the `ErrorRenderer` instead of a theme.
