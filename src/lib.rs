pub mod auth;
pub mod models;

pub trait Mentionable {
    fn to_mention(&self) -> String;
}
