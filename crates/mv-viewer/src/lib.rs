#![forbid(unsafe_code)]

//! The viewer: the state, its reducer, its reads, its words, and the plane
//! scene and motion it shows.

pub mod apply;
pub mod copy;
pub mod equation;
pub mod explainer;
pub mod form;
pub mod hints;
pub mod motion;
pub mod plane_scene;
pub mod selectors;
pub mod state;

pub use apply::*;
pub use copy::*;
pub use equation::*;
pub use explainer::*;
pub use form::*;
pub use hints::*;
pub use motion::*;
pub use plane_scene::*;
pub use selectors::*;
pub use state::*;
