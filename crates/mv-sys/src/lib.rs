//! Raw FFI surface for the C libraries the Rust port builds on.
//!
//! Ported from the Odin app's `vendor/` bindings: `clay/clay.odin` (Clay
//! `e6cc369`), `nanovg/nanovg.odin`'s API plus `nanovg/gl/gl.odin` (upstream
//! NanoVG `ce3bf74` with the Odin port's patches re-applied in C), and
//! `kb_text_shape/kb_text_shape_{procs,types}.odin` (kb_text_shape 2.21).
//! The C sources themselves land in `vendor/` with `t0c-a`/`t0c-b`; this
//! crate only declares the surface, and `build.rs` compiles nothing yet.
//!
//! Layout rules (the calls, recorded in
//! `.local/briefs/progress/t0c-surface.md`):
//! - Every struct that crosses the FFI is `#[repr(C)]`, with Odin/C field
//!   order kept; integer widths follow the C headers (`c_int` for C `int`,
//!   even where the Odin port's `int` is 64-bit), and are checked by the
//!   `csrc/sizes.c` test `t0c-a`/`t0c-b` write.
//! - Odin/C names are kept verbatim (camelCase fields, SCREAMING variants)
//!   so the declaration table greps one-to-one; each module carries the
//!   naming `allow`s. The CamelCase renaming of plan §1.3 applies to the
//!   app-package ports, not to this surface.
//! - `unsafe` lives here (and in the wrappers and platform files) only.
//!   Nullable C callbacks and out-pointers are `Option<...>`; nothing here
//!   is ` Send`/`Sync` by default.

pub mod clay;
pub mod kb;
pub mod kb_safe;
pub mod nvg;
