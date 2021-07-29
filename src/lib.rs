#![deny(warnings)]
#![deny(clippy::dbg_macro, clippy::todo, clippy::unimplemented)]
mod config;
mod config_loader;
#[cfg(test)]
mod tests;

pub use config::Config;
