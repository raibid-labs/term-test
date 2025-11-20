//! Integration tests module.

mod basic;
mod errors;
mod process;

#[cfg(feature = "sixel")]
mod sixel;

#[cfg(feature = "bevy")]
mod bevy;
