#![forbid(unsafe_code)]

//! The page's one number rule (Odin package `numfmt`, `src/numfmt/number.odin`):
//! every value a part shows prints a fixed number of decimals, through here.
//! Follows JavaScript's number semantics where the React code leans on them.

pub mod number;

pub use number::*;
