/// A State carrying Bot Data: the bot's own typed configuration and resources, built once and
/// shared by every Interaction (ADR 0014). Every Adapter's State implements it
/// (`composure_adapter_tokio::Host<T>`, `composure_adapter_cloudflare::Worker<T>`, with
/// `Data = T`), so a Command written against it runs on any of them:
///
/// ```
/// # #[cfg(feature = "macros")] mod example {
/// use composure::{command, command::{Ctx, HasData}};
///
/// pub struct Config {
///     pub greeting: String,
/// }
///
/// /// Greet with the configured greeting
/// #[command]
/// fn greet(ctx: Ctx<impl HasData<Data = Config>>) -> String {
///     format!("{} from Composure!", ctx.state().data().greeting)
/// }
/// # }
/// # fn main() {}
/// ```
///
/// When a platform's Bot Data holds more than a Command needs, bound on a part of it instead,
/// `Ctx<impl HasData<Data: AsRef<Config>>>`, and implement `AsRef<Config>` for each Bot Data
/// type.
pub trait HasData {
    /// The Bot Data
    type Data: ?Sized;

    /// The Bot Data
    fn data(&self) -> &Self::Data;
}
