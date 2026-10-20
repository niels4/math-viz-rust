//! Safe shaping sessions over [`kb`] (kb_text_shape 2.21).
//!
//! `mv-text` forbids `unsafe`, so the shape sequence lives here: context
//! create/destroy, font load/validity/info/free, font push/pop,
//! begin/end, feature push/pop, UTF-8 shaping, run and glyph iteration,
//! codepoint-source lookup. The raw-pointer handling stays inside these
//! wrappers; callers only see owned values, shared or `&mut` borrows, and
//! the pushed/popped font pointers the Odin call sequence ignores.
//!
//! Every item below is safe to call: fonts come from [`font_from_memory`]
//! (the shaper reports load failures through the font's error state, read
//! with [`font_is_valid`]); runs borrow the shaping context through
//! [`Run`], so a run cannot outlive the context that produced it (keeping a
//! [`kb::run`] past `end` or drop would be a use-after-free in safe code,
//! which is why the raw run never escapes); glyph views are owned copies.

use core::ffi::{c_int, c_void};
use core::marker::PhantomData;

use crate::kb;

/// The default allocator: C `malloc`/`free`. The C default allocator needs
/// setup the call sequence never provides, so shaping uses this instead.
unsafe extern "C" fn malloc_allocator(_data: *mut c_void, op: *mut kb::allocator_op) {
    unsafe {
        match (*op).Kind {
            kb::allocator_op_kind::ALLOCATE => {
                (*op).op.Allocate.Pointer = libc::malloc((*op).op.Allocate.Size as usize);
            }
            kb::allocator_op_kind::FREE => {
                libc::free((*op).op.Free.Pointer);
            }
            _ => {}
        }
    }
}

/// An owned shaping context; destroyed on drop.
pub struct ShapeContext {
    raw: *mut kb::shape_context,
}

impl ShapeContext {
    /// Create a shaping context with the default allocator; `None` on failure.
    pub fn new() -> Option<Self> {
        let raw =
            unsafe { kb::kbts_CreateShapeContext(Some(malloc_allocator), core::ptr::null_mut()) };
        (!raw.is_null()).then(|| Self { raw })
    }

    /// Push a font loaded by [`font_from_memory`]; returns the pushed pointer.
    pub fn push_font(&mut self, font: &mut kb::font) -> *mut kb::font {
        unsafe { kb::kbts_ShapePushFont(self.raw, font) }
    }

    /// Pop the top font; returns the popped pointer.
    pub fn pop_font(&mut self) -> *mut kb::font {
        unsafe { kb::kbts_ShapePopFont(self.raw) }
    }

    /// Begin a shaping pass.
    pub fn begin(&mut self, dir: kb::direction, lang: kb::language) {
        unsafe { kb::kbts_ShapeBegin(self.raw, dir, lang) }
    }

    /// End a shaping pass.
    pub fn end(&mut self) {
        unsafe { kb::kbts_ShapeEnd(self.raw) }
    }

    /// Turn an OpenType feature on (`value != 0`) or off.
    pub fn push_feature(&mut self, tag: kb::feature_tag, value: c_int) {
        unsafe { kb::kbts_ShapePushFeature(self.raw, tag, value) }
    }

    /// Pop a feature pushed by [`push_feature`].
    pub fn pop_feature(&mut self, tag: kb::feature_tag) {
        unsafe { kb::kbts_ShapePopFeature(self.raw, tag) };
    }

    /// Shape UTF-8 text; user ids follow `mode`.
    pub fn shape_utf8(&mut self, text: &str, mode: kb::user_id_generation_mode) {
        unsafe {
            kb::kbts_ShapeUtf8(
                self.raw,
                text.as_ptr() as *mut u8,
                text.len() as c_int,
                mode,
            )
        };
    }

    /// The next shaped run; `None` when shaping is done.
    ///
    /// Takes `&self`: the run borrows the context, so it cannot outlive it,
    /// while [`shape_codepoint`][Self::shape_codepoint] stays callable on the
    /// same shared borrow for each glyph's source lookup.
    pub fn shape_run(&self) -> Option<Run<'_>> {
        let mut run: kb::run = unsafe { core::mem::zeroed() };
        let ok = unsafe { kb::kbts_ShapeRun(self.raw, &mut run) };
        (ok != 0).then_some(Run {
            run,
            ctx: PhantomData,
        })
    }

    /// The codepoint behind a glyph's user id (the run's source index).
    ///
    /// Takes `&self` for the same reason as [`shape_run`][Self::shape_run]:
    /// the source lookup runs while a [`Run`] is alive.
    pub fn shape_codepoint(&self, index: c_int) -> (kb::shape_codepoint, u32) {
        let mut out: kb::shape_codepoint = unsafe { core::mem::zeroed() };
        let n = unsafe { kb::kbts_ShapeGetShapeCodepoint(self.raw, index, &mut out) };
        (out, n)
    }
}

impl Drop for ShapeContext {
    fn drop(&mut self) {
        unsafe { kb::kbts_DestroyShapeContext(self.raw) };
    }
}

/// Load a font from memory with the default allocator.
pub fn font_from_memory(data: &[u8], index: c_int) -> kb::font {
    unsafe { kb::FontFromMemory(data, index, Some(malloc_allocator), core::ptr::null_mut()) }
}

/// Whether the shaper can read the font.
pub fn font_is_valid(font: &mut kb::font) -> bool {
    unsafe { kb::kbts_FontIsValid(font) != 0 }
}

/// Metrics and units-per-em for a font.
pub fn font_info(font: &mut kb::font) -> kb::font_info2_1 {
    let mut info: kb::font_info2_1 = unsafe { core::mem::zeroed() };
    info.FontInfo2.Size = size_of::<kb::font_info2_1>() as u32;
    unsafe { kb::kbts_GetFontInfo2(font, &mut info.FontInfo2) };
    info
}

/// Free a font loaded by [`font_from_memory`].
pub fn free_font(font: &mut kb::font) {
    unsafe { kb::kbts_FreeFont(font) };
}

/// One shaped glyph, copied out of the run: safe to keep.
pub struct ShapedGlyph {
    /// The glyph index in its font.
    pub id: u16,
    /// Horizontal advance in font units.
    pub advance_x: i32,
    /// Pen offset in font units, x along the baseline.
    pub offset_x: i32,
    /// Pen offset in font units, y up.
    pub offset_y: i32,
    /// Whether a mark attached to its base (takes no spacing of its own).
    pub attached: bool,
    /// Source index (or codepoint index) for [`ShapeContext::shape_codepoint`].
    pub user_id_or_codepoint_index: c_int,
}

/// One shaped run: the C run with its glyph iterator, borrowing the shaping
/// context that produced it.
///
/// The borrow closes the use-after-free the review found: a `Copy` run kept
/// past `end` or drop would dereference run-internal pointers in safe code
/// through the old free `glyph_next`. Here the iterator only moves through
/// [`Run::glyph_next`], and the run cannot outlive its context.
pub struct Run<'ctx> {
    run: kb::run,
    ctx: PhantomData<&'ctx ShapeContext>,
}

impl<'ctx> Run<'ctx> {
    /// The shaping font this run came from, for the caller's file lookup.
    pub fn font(&self) -> *const kb::font {
        self.run.Font
    }

    /// The next glyph of the run's iterator; `None` when exhausted.
    pub fn glyph_next(&mut self) -> Option<ShapedGlyph> {
        let (g, ok) = unsafe { kb::GlyphIteratorNext(&mut self.run.Glyphs) };
        if !ok || g.is_null() {
            return None;
        }
        let g = unsafe { &*g };
        Some(ShapedGlyph {
            id: g.Id,
            advance_x: g.AdvanceX,
            offset_x: g.OffsetX,
            offset_y: g.OffsetY,
            attached: !g.AttachGlyph.is_null(),
            user_id_or_codepoint_index: g.UserIdOrCodepointIndex,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kb::{direction, language, user_id_generation_mode};

    const SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/WorkSans-SemiBold.ttf");

    /// The borrow-tied run shapes "To": two glyphs with source-index user
    /// ids 0 and 1, no second run, and the font pops after.
    #[test]
    fn runs_borrow_the_context_and_shape_to() {
        let mut ctx = ShapeContext::new().expect("a shape context");
        let mut font = font_from_memory(SEMIBOLD, 0);
        assert!(font_is_valid(&mut font));
        ctx.push_font(&mut font);
        ctx.begin(direction::KBTS_DIRECTION_LTR, language::DONT_KNOW);
        ctx.shape_utf8("To", user_id_generation_mode::SOURCE_INDEX);
        ctx.end();
        let mut run = ctx.shape_run().expect("one run for one font");
        let mut ids = Vec::new();
        let mut advance = 0;
        while let Some(g) = run.glyph_next() {
            let (cp, _) = ctx.shape_codepoint(g.user_id_or_codepoint_index);
            ids.push(cp.UserId);
            advance += g.advance_x;
        }
        assert_eq!(ids, [0, 1]);
        assert!(advance > 0);
        assert!(ctx.shape_run().is_none());
        ctx.pop_font();
        free_font(&mut font);
    }
}
