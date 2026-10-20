//! Verifying that requests come from Discord: every Interaction and Webhook Event is signed with
//! Ed25519 over `timestamp || body`, checked against the application's public key. Adapters do
//! this for you (`adapter::verify`, with the `rest` feature); use [`PublicKey`] directly
//! when writing one.

mod validate;

pub use validate::*;
