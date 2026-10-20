#![forbid(unsafe_code)]

//! The plane: the viewport, the grid, the sampled traces, label placement,
//! plates, gestures, arrivals, marks, and the painters that draw them.

pub mod arrivals;
pub mod chrome;
pub mod dash;
pub mod faces;
pub mod gestures;
pub mod grid;
pub mod marks;
pub mod mini;
pub mod paint;
pub mod placement;
pub mod plates;
pub mod rect;
pub mod scene;
pub mod trace;
pub mod viewport;

pub use arrivals::*;
pub use chrome::*;
pub use dash::*;
pub use faces::*;
pub use gestures::*;
pub use grid::*;
pub use marks::*;
pub use mini::*;
pub use paint::*;
pub use placement::*;
pub use plates::*;
pub use rect::*;
pub use scene::*;
pub use trace::*;
pub use viewport::*;
