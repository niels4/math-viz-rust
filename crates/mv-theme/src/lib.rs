#![forbid(unsafe_code)]

//! The page's appearance (Odin package `theme`): the six themes as data, their tokens resolved to
//! drawing colours in sRGB or Display P3, and the stored preferences that
//! pick the theme shown. Pure: the platform layer does IO.

pub mod color;
pub mod prefs;
pub mod theme;
pub mod themes;

pub use color::*;
pub use prefs::*;
pub use theme::*;
pub use themes::{
    DEFINITIONS, Definition, Mix, ReadoutFont, ShadowDefinition, ThemeDef, Token, Value,
};
