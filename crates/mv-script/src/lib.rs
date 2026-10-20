#![forbid(unsafe_code)]

//! The harness's action language (Odin package `script`, `src/script/script.odin`):
//! the scripts that put the reference and the builds in the same state. Twin
//! of `tools/src/script.py`: change the two together.

pub mod script;

pub use script::*;
