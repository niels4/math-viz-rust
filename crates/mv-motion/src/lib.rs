#![forbid(unsafe_code)]
//! The page's motion maths (Odin package `motion`): damped springs in closed
//! form, CSS's cubic-bezier() and linear() timing functions, the motion tokens,
//! CSS transitions as the browsers run them, and an overlay's presence.

pub mod bezier;
pub mod presence;
pub mod spring;
pub mod tokens;
pub mod transition;

pub use bezier::*;
pub use presence::*;
pub use spring::*;
pub use tokens::*;
pub use transition::*;
