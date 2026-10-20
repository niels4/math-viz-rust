#![forbid(unsafe_code)]

//! The page's text: the embedded faces, shaping with the fonts' kerning,
//! line boxes, typeset lines, fitting, truncation, wrapping, and drawing.

pub mod draw;
pub mod fit;
pub mod fonts;
pub mod line;
pub mod overflow;
pub mod shape;
pub mod typeset;
pub mod wrap;

pub use draw::*;
pub use fit::*;
pub use fonts::*;
pub use line::*;
pub use overflow::*;
pub use shape::*;
pub use typeset::*;
pub use wrap::*;
