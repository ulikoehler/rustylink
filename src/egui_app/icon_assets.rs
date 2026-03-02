#![cfg(feature = "egui")]

use std::borrow::Cow;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "icons"]
pub struct EmbeddedIcons;

pub fn get(path: &str) -> Option<Cow<'static, [u8]>> {
    EmbeddedIcons::get(path).map(|f| f.data)
}
