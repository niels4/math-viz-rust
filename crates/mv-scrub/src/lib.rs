#![forbid(unsafe_code)]

//! The jog maths of anything dragged like a ruler (Odin package `scrub`):
//! relative scrubbing with no min or max, and the jog ruler's tape marks.

pub mod ruler;
pub mod scrub;

pub use ruler::*;
pub use scrub::*;
