//! kb_text_shape 2.21 (`vendor/kb_text_shape/kb_text_shape_procs.odin`
//! and `kb_text_shape_types.odin`).
//!
//! The `kbts_*` symbols are the real C entry points (declared `extern`
//! below with the binding's exact signatures). The Odin-level `Shape*`,
//! `Break*`, `LoadFont` etc. conveniences that take slices/strings become
//! the `unsafe fn` wrappers at the bottom of this file, with real
//! forwarding bodies. Two Odin items have no Rust counterpart (recorded
//! exclusions, see the progress file): `AllocatorFromOdinAllocator`
//! (bridges Odin's own allocator; Rust passes allocators explicitly) and
//! the typeless `WordBreaks` field, which the C header pins as
//! `kbts_u16 WordBreaks; // u4x4`.
//!
//! Integer widths mirror the binding (`rune` is `i32`, `b32` is `u32`,
//! `un` is `usize`); every one of those matches the C header (`int`,
//! `int32_t` codepoints, `kbts_un`), checked by `t0c`'s size test.

#![allow(non_camel_case_types, non_snake_case)]

use core::ffi::{c_char, c_int, c_void};

#[path = "kb_generated.rs"]
mod kb_generated;
pub use kb_generated::*;

// --- Opaque C structs (pointers only in the binding) ------------------------------

macro_rules! opaque {
    ($(#[$m:meta])* $name:ident, $odin:literal) => {
        $(#[$m])*
        #[repr(C)]
        #[derive(Debug)]
        pub struct $name {
            _private: [u8; 0],
        }
    };
}

opaque!(
    /// Ported from `kb_text_shape_types.odin:1759` (`_gdef`).
    _gdef, "_gdef"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1760` (`_cmap_14`).
    _cmap_14, "_cmap_14"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1761` (`_gsub_gpos`).
    _gsub_gpos, "_gsub_gpos"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1762` (`_maxp`).
    _maxp, "_maxp"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1763` (`_hea`).
    _hea, "_hea"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1764` (`shaper_properties`).
    shaper_properties, "shaper_properties"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1765` (`_feature`).
    _feature, "_feature"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1766` (`_head`).
    _head, "_head"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1767` (`_langsys`).
    _langsys, "_langsys"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1768` (`shape_config`).
    shape_config, "shape_config"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1769` (`glyph_config`);
    /// empty in the binding, used through pointers only.
    glyph_config, "glyph_config"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1770` (`bucketed_glyph`);
    /// empty in the binding, used through pointers only.
    bucketed_glyph, "bucketed_glyph"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1771` (`shape_context`).
    shape_context, "shape_context"
);
opaque!(
    /// Ported from `kb_text_shape_types.odin:1772` (`shape_scratchpad`).
    shape_scratchpad, "shape_scratchpad"
);

// --- Allocator -------------------------------------------------------------------------

/// Ported from `kb_text_shape_types.odin:1779` (`allocator_op_free`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct allocator_op_free {
    pub Pointer: *mut c_void,
}

/// Ported from `kb_text_shape_types.odin:1774` (`allocator_op_allocate`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct allocator_op_allocate {
    pub Pointer: *mut c_void,
    pub Size: u32,
}

/// Ported from `kb_text_shape_types.odin:1786` (`allocator_op`'s `#raw_union`).
#[repr(C)]
#[derive(Clone, Copy)]
pub union AllocatorOp {
    pub Allocate: allocator_op_allocate,
    pub Free: allocator_op_free,
}

/// Ported from `kb_text_shape_types.odin:1783` (`allocator_op`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct allocator_op {
    pub Kind: allocator_op_kind,
    pub op: AllocatorOp,
}

/// Ported from `kb_text_shape_types.odin:1792` (`allocator_function`).
pub type allocator_function =
    Option<unsafe extern "C" fn(Data: *mut c_void, Op: *mut allocator_op)>;

// --- Font loading -------------------------------------------------------------------------

/// Ported from `kb_text_shape_types.odin:1794` (`lookup_subtable_info`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct lookup_subtable_info {
    pub MinimumBacktrackPlusOne: u16,
    pub MinimumFollowupPlusOne: u16,
}

/// Ported from `kb_text_shape_types.odin:1799` (`blob_table`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct blob_table {
    pub OffsetFromStartOfFile: u32,
    pub Length: u32,
}

/// Ported from `kb_text_shape_types.odin:1804` (`load_font_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct load_font_state {
    pub FontData: *mut c_void,
    pub FontDataSize: u32,
    pub Tables: [blob_table; 13],
    pub LookupCount: u32,
    pub LookupSubtableCount: u32,
    pub GlyphCount: u32,
    pub ScratchSize: u32,
    pub GlyphLookupMatrixSizeInBytes: u32,
    pub GlyphLookupSubtableMatrixSizeInBytes: u32,
    pub TotalSize: u32,
}

/// Ported from `kb_text_shape_types.odin:1819` (`blob_header`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct blob_header {
    pub Magic: u32,
    pub Version: u32,
    pub LookupCount: u32,
    pub LookupSubtableCount: u32,
    pub GlyphCount: u32,
    pub GposLookupIndexOffset: u32,
    pub GlyphLookupMatrixOffsetFromStartOfFile: u32,
    pub GlyphLookupSubtableMatrixOffsetFromStartOfFile: u32,
    pub LookupSubtableIndexOffsetsOffsetFromStartOfFile: u32,
    pub SubtableInfosOffsetFromStartOfFile: u32,
    pub Tables: [blob_table; 13],
}

/// Ported from `kb_text_shape_types.odin:1837` (`font`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct font {
    pub Allocator: allocator_function,
    pub AllocatorData: *mut c_void,
    pub Blob: *mut blob_header,
    pub Cmap: *mut u16,
    pub Cmap14: *mut _cmap_14,
    pub ShapingTables: [*mut _gsub_gpos; 2],
    pub UserData: *mut c_void,
    pub Error: load_font_error,
}

/// Ported from `kb_text_shape_types.odin:1852` (`font_info`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct font_info {
    pub Strings: [*const c_char; 13],
    pub StringLengths: [u16; 13],
    pub StyleFlags: font_style_flags,
    pub Weight: font_weight,
    pub Width: font_width,
}

/// Ported from `kb_text_shape_types.odin:1860` (`font_info2`;
/// Odin's `using FontInfo` is flattened: the field keeps its Odin name).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct font_info2 {
    pub Size: u32,
    pub FontInfo: font_info,
}

/// Ported from `kb_text_shape_types.odin:1864` (`font_info2_1`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct font_info2_1 {
    pub FontInfo2: font_info2,
    pub UnitsPerEm: u16,
    pub XMin: i16,
    pub YMin: i16,
    pub XMax: i16,
    pub YMax: i16,
    pub Ascent: i16,
    pub Descent: i16,
    pub LineGap: i16,
}

/// Ported from `kb_text_shape_types.odin:1870` (`font_info2_2`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct font_info2_2 {
    pub FontInfo2_1: font_info2_1,
    pub CapitalHeight: i16,
}

/// Ported from `kb_text_shape_types.odin:1878` (`feature_override`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct feature_override {
    pub Tag: feature_tag,
    pub Value: c_int,
}

// --- Segmentation ---------------------------------------------------------------------------

/// Ported from `kb_text_shape_types.odin:1883` (`break_type`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct break_type {
    pub Position: c_int,
    pub Flags: break_flags,
    pub Direction: direction,
    pub ParagraphDirection: direction,
    pub Script: script,
}

/// Ported from `kb_text_shape_types.odin:1893` (`bracket`).
///
/// The two `bit_field u8` members are single-byte storage; their layout
/// (`Direction: direction | 8`, `Script: script | 8`) is decoded by the
/// text port's body.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct bracket {
    pub Codepoint: i32,
    pub Position: u32,
    pub DirectionBitField: u8,
    pub ScriptBitField: u8,
}

/// Ported from `kb_text_shape_types.odin:1908` (`break_state`).
///
/// Every `bit_field` member is plain integer storage of the named width;
/// `WordBreaks` is `u16` per the C header (`kbts_u16 WordBreaks; // u4x4`),
/// which pins down the Odin binding's typeless field. The `using`
/// bit-field keeps its Odin name.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct break_state {
    pub Breaks: [break_type; 8],
    pub BreakCount: u32,
    pub ParagraphDirection: direction,
    pub UserParagraphDirection: direction,
    pub CurrentPosition: u32,
    pub ParagraphStartPosition: u32,
    pub LastScriptBreakPosition: u32,
    pub LastDirectionBreakPosition: u32,
    pub LastScriptBreakScript: u8,
    pub LastDirectionBreakDirection: u8,
    pub ScriptPositionOffset: i16,
    pub ScriptCount: u32,
    pub ScriptSet: [u8; 23],
    pub Brackets: [bracket; 64],
    pub BracketCount: u32,
    pub Flags: break_state_flags,
    pub FlagState: u32,
    pub PositionOffset2: i16,
    pub PositionOffset3: i16,
    pub WordBreakHistory: u32,
    pub WordBreaks: u16,
    pub WordUnbreaks: u16,
    pub WordBreak2PositionOffset: i16,
    pub LineBreaks: u64,
    pub LineUnbreaksAsync: u64,
    pub LineUnbreaks: u64,
    pub LineBreakHistory: u32,
    pub LineBreak2PositionOffset: i16,
    pub LineBreak3PositionOffset: i16,
    pub LastDirectionBitField: u8,
    pub BidirectionalClass2: u8,
    pub BidirectionalClass1: u8,
    pub Bidirectional1PositionOffset: i16,
    pub Bidirectional2PositionOffset: i16,
    pub JapaneseLineBreakStyle: japanese_line_break_style,
    pub ConfigFlags: break_config_flags,
    pub GraphemeBreakState: u8,
    pub LastLineBreakClass: u8,
    pub LastWordBreakClass: u8,
    pub LastWordBreakClassIncludingIgnored: u8,
}

/// Ported from `kb_text_shape_types.odin:2001` (`decode`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct decode {
    pub Codepoint: i32,
    pub SourceCharactersConsumed: c_int,
    pub Valid: u32,
}

/// Ported from `kb_text_shape_types.odin:2008` (`encode_utf8`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct encode_utf8 {
    pub Encoded: [u8; 4],
    pub EncodedLength: c_int,
    pub Valid: u32,
}

// --- Shaping -------------------------------------------------------------------------------

/// Ported from `kb_text_shape_types.odin:2014` (`glyph_classes`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct glyph_classes {
    pub Class: u16,
    pub MarkAttachmentClass: u16,
}

/// Ported from `kb_text_shape_types.odin:2019` (`glyph`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct glyph {
    pub Prev: *mut glyph,
    pub Next: *mut glyph,
    pub Codepoint: i32,
    pub Id: u16,
    pub Uid: u16,
    pub UserIdOrCodepointIndex: c_int,
    pub OffsetX: i32,
    pub OffsetY: i32,
    pub AdvanceX: i32,
    pub AdvanceY: i32,
    pub AttachGlyph: *mut glyph,
    pub Config: *mut glyph_config,
    pub Decomposition: u64,
    pub Classes: glyph_classes,
    pub Flags: glyph_flags,
    pub ParentInfo: u32,
    pub Bucketed: *mut bucketed_glyph,
    pub SortKey: u32,
    pub SortKeyInterval: u32,
    pub BucketedBucketIndex: u16,
    pub LigatureUid: u16,
    pub LigatureComponentIndexPlusOne: u16,
    pub LigatureComponentCount: u16,
    pub JoiningFeature: joining_feature,
    pub JoiningType: unicode_joining_type,
    pub UnicodeFlags: unicode_flags,
    pub SyllabicClass: u8,
    pub SyllabicPosition: u8,
    pub UseClass: u8,
    pub CombiningClass: u8,
    pub MarkOrdering: u8,
}

/// Ported from `kb_text_shape_types.odin:2095` (`shape_codepoint`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct shape_codepoint {
    pub Font: *mut font,
    pub FeatureOverrides: *const feature_override,
    pub FeatureOverrideCount: c_int,
    pub Codepoint: i32,
    pub UserId: c_int,
    pub BreakFlags: break_flags,
    pub Script: script,
    pub Direction: direction,
    pub ParagraphDirection: direction,
}

/// Ported from `kb_text_shape_types.odin:2110` (`shape_codepoint_iterator`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct shape_codepoint_iterator {
    pub Codepoint: *mut shape_codepoint,
    pub Context: *mut shape_context,
    pub EndBlockIndex: u32,
    pub OnePastLastCodepointIndex: u32,
    pub BlockIndex: u32,
    pub CodepointIndex: u32,
    pub CurrentBlockCodepointCount: u32,
    pub FlatCodepointIndex: u32,
}

/// Ported from `kb_text_shape_types.odin:2122` (`glyph_iterator`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct glyph_iterator {
    pub GlyphStorage: *mut glyph_storage,
    pub CurrentGlyph: *mut glyph,
    pub LastAdvanceX: c_int,
    pub X: c_int,
    pub Y: c_int,
}

/// Ported from `kb_text_shape_types.odin:2131` (`arena_block_header`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct arena_block_header {
    pub Prev: *mut arena_block_header,
    pub Next: *mut arena_block_header,
}

/// Ported from `kb_text_shape_types.odin:2136` (`arena`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct arena {
    pub Allocator: allocator_function,
    pub AllocatorData: *mut c_void,
    pub BlockSentinel: arena_block_header,
    pub FreeBlockSentinel: arena_block_header,
    pub Error: u32,
}

/// Ported from `kb_text_shape_types.odin:2146` (`glyph_storage`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct glyph_storage {
    pub Arena: arena,
    pub GlyphSentinel: glyph,
    pub FreeGlyphSentinel: glyph,
    pub Error: u32,
}

/// Ported from `kb_text_shape_types.odin:2155` (`glyph_parent`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct glyph_parent {
    pub Codepoint: i32,
    pub Codepoint1: i32,
}

/// Ported from `kb_text_shape_types.odin:2160` (`font_coverage_test`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct font_coverage_test {
    pub Font: *mut font,
    pub BaseCodepoint: i32,
    pub CurrentBaseError: u32,
    pub Error: u32,
    pub BaseParents: [glyph_parent; 19],
    pub BaseParentCount: u32,
}

/// Ported from `kb_text_shape_types.odin:2171` (`run`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct run {
    pub Font: *mut font,
    pub Script: script,
    pub ParagraphDirection: direction,
    pub Direction: direction,
    pub Flags: break_flags,
    pub Glyphs: glyph_iterator,
}

// --- The C entry points -------------------------------------------------------------------------

unsafe extern "C" {
    // Context API (`kb_text_shape_procs.odin:28-49`).
    /// Ported from `kb_text_shape_procs.odin:31` (`SizeOfShapeContext`).
    pub unsafe fn kbts_SizeOfShapeContext() -> c_int;
    /// Ported from `kb_text_shape_procs.odin:32` (`PlaceShapeContext2`).
    pub unsafe fn kbts_PlaceShapeContext2(
        Allocator: allocator_function,
        AllocatorData: *mut c_void,
        Memory: *mut c_void,
        Flags: shape_context_flags,
    ) -> *mut shape_context;
    /// Ported from `kb_text_shape_procs.odin:33` (`PlaceShapeContext`).
    pub unsafe fn kbts_PlaceShapeContext(
        Allocator: allocator_function,
        AllocatorData: *mut c_void,
        Memory: *mut c_void,
    ) -> *mut shape_context;
    /// Ported from `kb_text_shape_procs.odin:34` (`CreateShapeContext2`).
    pub unsafe fn kbts_CreateShapeContext2(
        Allocator: allocator_function,
        AllocatorData: *mut c_void,
        Flags: shape_context_flags,
    ) -> *mut shape_context;
    /// Ported from `kb_text_shape_procs.odin:35` (`CreateShapeContext`).
    pub unsafe fn kbts_CreateShapeContext(
        Allocator: allocator_function,
        AllocatorData: *mut c_void,
    ) -> *mut shape_context;
    /// Ported from `kb_text_shape_procs.odin:36` (`DestroyShapeContext`).
    pub unsafe fn kbts_DestroyShapeContext(Context: *mut shape_context);
    /// Ported from `kb_text_shape_procs.odin:37` (`ShapePushFont`).
    pub unsafe fn kbts_ShapePushFont(Context: *mut shape_context, Font: *mut font) -> *mut font;
    /// Ported from `kb_text_shape_procs.odin:38` (`ShapePopFont`).
    pub unsafe fn kbts_ShapePopFont(Context: *mut shape_context) -> *mut font;
    /// Ported from `kb_text_shape_procs.odin:39` (`ShapeBegin`).
    pub unsafe fn kbts_ShapeBegin(
        Context: *mut shape_context,
        ParagraphDirection: direction,
        Language: language,
    );
    /// Ported from `kb_text_shape_procs.odin:40` (`ShapeEnd`).
    pub unsafe fn kbts_ShapeEnd(Context: *mut shape_context);
    /// Ported from `kb_text_shape_procs.odin:41` (`ShapePushFeature`).
    pub unsafe fn kbts_ShapePushFeature(
        Context: *mut shape_context,
        FeatureTag: feature_tag,
        Value: c_int,
    );
    /// Ported from `kb_text_shape_procs.odin:42` (`ShapePopFeature`).
    pub unsafe fn kbts_ShapePopFeature(Context: *mut shape_context, FeatureTag: feature_tag)
    -> u32;
    /// Ported from `kb_text_shape_procs.odin:43` (`ShapeCodepoint`).
    pub unsafe fn kbts_ShapeCodepoint(Context: *mut shape_context, Codepoint: i32);
    /// Ported from `kb_text_shape_procs.odin:44` (`ShapeCodepointWithUserId`).
    pub unsafe fn kbts_ShapeCodepointWithUserId(
        Context: *mut shape_context,
        Codepoint: i32,
        UserId: c_int,
    );
    /// Ported from `kb_text_shape_procs.odin:45` (`ShapeError`).
    pub unsafe fn kbts_ShapeError(Context: *mut shape_context) -> shape_error;
    /// Ported from `kb_text_shape_procs.odin:46` (`ShapeBeginManualRuns`).
    pub unsafe fn kbts_ShapeBeginManualRuns(Context: *mut shape_context);
    /// Ported from `kb_text_shape_procs.odin:47` (`ShapeNextManualRun`).
    pub unsafe fn kbts_ShapeNextManualRun(
        Context: *mut shape_context,
        Direction: direction,
        Script: script,
    );
    /// Ported from `kb_text_shape_procs.odin:48` (`ShapeEndManualRuns`).
    pub unsafe fn kbts_ShapeEndManualRuns(Context: *mut shape_context);
    /// Ported from `kb_text_shape_procs.odin:49` (`ShapeManualBreak`).
    pub unsafe fn kbts_ShapeManualBreak(Context: *mut shape_context);

    // Context API: codepoint iteration (`kb_text_shape_procs.odin:117-122`).
    /// Ported from `kb_text_shape_procs.odin:119` (`ShapeCurrentCodepointsIterator`).
    pub unsafe fn kbts_ShapeCurrentCodepointsIterator(
        Context: *mut shape_context,
    ) -> shape_codepoint_iterator;
    /// Ported from `kb_text_shape_procs.odin:120` (`ShapeCodepointIteratorIsValid`).
    pub unsafe fn kbts_ShapeCodepointIteratorIsValid(It: *mut shape_codepoint_iterator) -> u32;
    /// Ported from `kb_text_shape_procs.odin:121` (`ShapeGetShapeCodepoint`).
    pub unsafe fn kbts_ShapeGetShapeCodepoint(
        Context: *mut shape_context,
        CodepointIndex: c_int,
        Codepoint: *mut shape_codepoint,
    ) -> u32;

    // Direct API: fonts and configs (`kb_text_shape_procs.odin:139-175`).
    /// Ported from `kb_text_shape_procs.odin:142` (`FreeFont`).
    pub unsafe fn kbts_FreeFont(Font: *mut font);
    /// Ported from `kb_text_shape_procs.odin:143` (`FontIsValid`).
    pub unsafe fn kbts_FontIsValid(Font: *mut font) -> u32;
    /// Ported from `kb_text_shape_procs.odin:144` (`PlaceBlob`).
    pub unsafe fn kbts_PlaceBlob(
        Font: *mut font,
        State: *mut load_font_state,
        ScratchMemory: *mut c_void,
        OutputMemory: *mut c_void,
    ) -> load_font_error;
    /// Ported from `kb_text_shape_procs.odin:145` (`GetFontInfo`).
    pub unsafe fn kbts_GetFontInfo(Font: *mut font, Info: *mut font_info);
    /// Ported from `kb_text_shape_procs.odin:146` (`GetFontInfo2`).
    pub unsafe fn kbts_GetFontInfo2(Font: *mut font, Info: *mut font_info2);
    /// Ported from `kb_text_shape_procs.odin:149` (`SizeOfShapeConfig`).
    pub unsafe fn kbts_SizeOfShapeConfig(
        Font: *mut font,
        Script: script,
        Language: language,
    ) -> u32;
    /// Ported from `kb_text_shape_procs.odin:150` (`PlaceShapeConfig`).
    pub unsafe fn kbts_PlaceShapeConfig(
        Font: *mut font,
        Script: script,
        Language: language,
        Memory: *mut c_void,
    ) -> *mut shape_config;
    /// Ported from `kb_text_shape_procs.odin:151` (`CreateShapeConfig`).
    pub unsafe fn kbts_CreateShapeConfig(
        Font: *mut font,
        Script: script,
        Language: language,
        Allocator: allocator_function,
        AllocatorData: *mut c_void,
    ) -> *mut shape_config;
    /// Ported from `kb_text_shape_procs.odin:152` (`DestroyShapeConfig`).
    pub unsafe fn kbts_DestroyShapeConfig(Config: *mut shape_config);
    /// Ported from `kb_text_shape_procs.odin:155` (`InitializeGlyphStorage`).
    pub unsafe fn kbts_InitializeGlyphStorage(
        Storage: *mut glyph_storage,
        Allocator: allocator_function,
        AllocatorData: *mut c_void,
    ) -> u32;
    /// Ported from `kb_text_shape_procs.odin:156`
    /// (`InitializeGlyphStorageFixedMemory`).
    pub unsafe fn kbts_InitializeGlyphStorageFixedMemory(
        Storage: *mut glyph_storage,
        Memory: *mut c_void,
        MemorySize: c_int,
    ) -> u32;
    /// Ported from `kb_text_shape_procs.odin:157` (`PushGlyph`).
    pub unsafe fn kbts_PushGlyph(
        Storage: *mut glyph_storage,
        Font: *mut font,
        Codepoint: i32,
        Config: *mut glyph_config,
        UserId: c_int,
    ) -> *mut glyph;
    /// Ported from `kb_text_shape_procs.odin:158` (`ClearActiveGlyphs`).
    pub unsafe fn kbts_ClearActiveGlyphs(Storage: *mut glyph_storage);
    /// Ported from `kb_text_shape_procs.odin:159` (`FreeAllGlyphs`).
    pub unsafe fn kbts_FreeAllGlyphs(Storage: *mut glyph_storage);
    /// Ported from `kb_text_shape_procs.odin:160` (`CodepointToGlyph`).
    pub unsafe fn kbts_CodepointToGlyph(
        Font: *mut font,
        Codepoint: i32,
        Config: *mut glyph_config,
        UserId: c_int,
    ) -> glyph;
    /// Ported from `kb_text_shape_procs.odin:161` (`CodepointToGlyphId`).
    pub unsafe fn kbts_CodepointToGlyphId(Font: *mut font, Codepoint: i32) -> c_int;
    /// Ported from `kb_text_shape_procs.odin:162` (`ActiveGlyphIterator`).
    pub unsafe fn kbts_ActiveGlyphIterator(Storage: *mut glyph_storage) -> glyph_iterator;
    /// Ported from `kb_text_shape_procs.odin:166` (`DestroyGlyphConfig`).
    pub unsafe fn kbts_DestroyGlyphConfig(Config: *mut glyph_config);
    /// Ported from `kb_text_shape_procs.odin:171` (`SizeOfShapeScratchpad`).
    pub unsafe fn kbts_SizeOfShapeScratchpad(Config: *mut shape_config) -> usize;
    /// Ported from `kb_text_shape_procs.odin:172` (`PlaceShapeScratchpad`).
    pub unsafe fn kbts_PlaceShapeScratchpad(
        Config: *mut shape_config,
        Memory: *mut c_void,
        Allocator: allocator_function,
        AllocatorData: *mut c_void,
    ) -> shape_scratchpad;
    /// Ported from `kb_text_shape_procs.odin:173`
    /// (`PlaceShapeScratchpadFixedMemory`).
    pub unsafe fn kbts_PlaceShapeScratchpadFixedMemory(
        Config: *mut shape_config,
        Memory: *mut c_void,
        Size: c_int,
    ) -> shape_scratchpad;
    /// Ported from `kb_text_shape_procs.odin:174` (`CreateShapeScratchpad`).
    pub unsafe fn kbts_CreateShapeScratchpad(
        Config: *mut shape_config,
        Allocator: allocator_function,
        AllocatorData: *mut c_void,
    ) -> shape_scratchpad;
    /// Ported from `kb_text_shape_procs.odin:175` (`DestroyShapeScratchpad`).
    pub unsafe fn kbts_DestroyShapeScratchpad(Scratchpad: *mut shape_scratchpad);
    /// Ported from `kb_text_shape_procs.odin:246` (`GlyphIteratorIsValid`).
    pub unsafe fn kbts_GlyphIteratorIsValid(It: *mut glyph_iterator) -> u32;

    // Segmentation (`kb_text_shape_procs.odin:263-268`, `:329-337`).
    /// Ported from `kb_text_shape_procs.odin:265` (`BreakBegin`).
    pub unsafe fn kbts_BreakBegin(
        State: *mut break_state,
        ParagraphDirection: direction,
        JapaneseLineBreakStyle: japanese_line_break_style,
        ConfigFlags: break_config_flags,
    );
    /// Ported from `kb_text_shape_procs.odin:266` (`BreakAddCodepoint`).
    pub unsafe fn kbts_BreakAddCodepoint(
        State: *mut break_state,
        Codepoint: i32,
        PositionIncrement: c_int,
        EndOfText: u32,
    );
    /// Ported from `kb_text_shape_procs.odin:267` (`BreakEnd`).
    pub unsafe fn kbts_BreakEnd(State: *mut break_state);
    /// Ported from `kb_text_shape_procs.odin:331` (`FontCoverageTestBegin`).
    pub unsafe fn kbts_FontCoverageTestBegin(Test: *mut font_coverage_test, Font: *mut font);
    /// Ported from `kb_text_shape_procs.odin:332` (`FontCoverageTestCodepoint`).
    pub unsafe fn kbts_FontCoverageTestCodepoint(Test: *mut font_coverage_test, Codepoint: i32);
    /// Ported from `kb_text_shape_procs.odin:333` (`FontCoverageTestEnd`).
    pub unsafe fn kbts_FontCoverageTestEnd(Test: *mut font_coverage_test) -> u32;
    /// Ported from `kb_text_shape_procs.odin:335` (`ScriptDirection`).
    pub unsafe fn kbts_ScriptDirection(Script: script) -> direction;
    /// Ported from `kb_text_shape_procs.odin:336` (`ScriptIsComplex`).
    pub unsafe fn kbts_ScriptIsComplex(Script: script) -> u32;
    /// Ported from `kb_text_shape_procs.odin:337` (`ScriptTagToScript`).
    pub unsafe fn kbts_ScriptTagToScript(Tag: script_tag) -> script;

    // Slice-taking entry points: the true C signatures behind the Odin
    // wrappers below (`kb_text_shape_procs.odin:52-114`, `:125-132`,
    // `:178-257`, `:270-324`, `:340-393`).
    /// Ported from `kb_text_shape_procs.odin:55` (inner `kbts_PlaceShapeContextFixedMemory2`).
    pub unsafe fn kbts_PlaceShapeContextFixedMemory2(
        Memory: *mut c_void,
        Size: c_int,
        Flags: shape_context_flags,
    ) -> *mut shape_context;
    /// Ported from `kb_text_shape_procs.odin:63` (inner `kbts_PlaceShapeContextFixedMemory`).
    pub unsafe fn kbts_PlaceShapeContextFixedMemory(
        Memory: *mut c_void,
        Size: c_int,
    ) -> *mut shape_context;
    /// Ported from `kb_text_shape_procs.odin:71` (inner `kbts_ShapePushFontFromMemory`).
    pub unsafe fn kbts_ShapePushFontFromMemory(
        Context: *mut shape_context,
        Memory: *mut c_void,
        Size: c_int,
        FontIndex: c_int,
    ) -> *mut font;
    /// Ported from `kb_text_shape_procs.odin:79` (inner `kbts_ShapeRun`).
    pub unsafe fn kbts_ShapeRun(Context: *mut shape_context, Run: *mut run) -> u32;
    /// Ported from `kb_text_shape_procs.odin:89` (inner `kbts_ShapeUtf32`).
    pub unsafe fn kbts_ShapeUtf32(Context: *mut shape_context, Utf32: *mut i32, Length: c_int);
    /// Ported from `kb_text_shape_procs.odin:96` (inner `kbts_ShapeUtf32WithUserId`).
    pub unsafe fn kbts_ShapeUtf32WithUserId(
        Context: *mut shape_context,
        Utf32: *mut i32,
        Length: c_int,
        UserId: c_int,
        UserIdIncrement: c_int,
    );
    /// Ported from `kb_text_shape_procs.odin:105` (inner `kbts_ShapeUtf8`).
    pub unsafe fn kbts_ShapeUtf8(
        Context: *mut shape_context,
        Utf8: *mut u8,
        Length: c_int,
        UserIdGenerationMode: user_id_generation_mode,
    );
    /// Ported from `kb_text_shape_procs.odin:112` (inner `kbts_ShapeUtf8WithUserId`).
    pub unsafe fn kbts_ShapeUtf8WithUserId(
        Context: *mut shape_context,
        Utf8: *mut u8,
        Length: c_int,
        UserId: c_int,
        UserIdGenerationMode: user_id_generation_mode,
    );
    /// Ported from `kb_text_shape_procs.odin:128` (inner `kbts_ShapeCodepointIteratorNext`).
    pub unsafe fn kbts_ShapeCodepointIteratorNext(
        It: *mut shape_codepoint_iterator,
        Codepoint: *mut shape_codepoint,
        CodepointIndex: *mut c_int,
    ) -> u32;
    /// Ported from `kb_text_shape_procs.odin:181` (inner `kbts_LoadFont`).
    pub unsafe fn kbts_LoadFont(
        Font: *mut font,
        State: *mut load_font_state,
        FontData: *mut c_void,
        FontDataSize: c_int,
        FontIndex: c_int,
        ScratchSize: *mut c_int,
        OutputSize: *mut c_int,
    ) -> load_font_error;
    /// Ported from `kb_text_shape_procs.odin:192` (inner `kbts_FontCount`).
    pub unsafe fn kbts_FontCount(FileData: *mut c_void, FileSize: c_int) -> c_int;
    /// Ported from `kb_text_shape_procs.odin:201` (inner `kbts_FontFromMemory`).
    pub unsafe fn kbts_FontFromMemory(
        FileData: *mut c_void,
        FileSize: c_int,
        FontIndex: c_int,
        Allocator: allocator_function,
        AllocatorData: *mut c_void,
    ) -> font;
    /// Ported from `kb_text_shape_procs.odin:211` (inner `kbts_ShapeDirect`).
    pub unsafe fn kbts_ShapeDirect(
        Scratchpad: *mut shape_scratchpad,
        Storage: *mut glyph_storage,
        RunDirection: direction,
        Output: *mut glyph_iterator,
    ) -> shape_error;
    /// Ported from `kb_text_shape_procs.odin:222` (inner `kbts_SizeOfGlyphConfig`).
    pub unsafe fn kbts_SizeOfGlyphConfig(
        ShapeConfig: *mut shape_config,
        Overrides: *const feature_override,
        OverrideCount: c_int,
    ) -> c_int;
    /// Ported from `kb_text_shape_procs.odin:230` (inner `kbts_PlaceGlyphConfig`).
    pub unsafe fn kbts_PlaceGlyphConfig(
        ShapeConfig: *mut shape_config,
        Overrides: *const feature_override,
        OverrideCount: c_int,
        Memory: *mut c_void,
    ) -> *mut glyph_config;
    /// Ported from `kb_text_shape_procs.odin:239` (inner `kbts_CreateGlyphConfig`).
    pub unsafe fn kbts_CreateGlyphConfig(
        ShapeConfig: *mut shape_config,
        Overrides: *const feature_override,
        OverrideCount: c_int,
        Allocator: allocator_function,
        AllocatorData: *mut c_void,
    ) -> *mut glyph_config;
    /// Ported from `kb_text_shape_procs.odin:253` (inner `kbts_GlyphIteratorNext`).
    pub unsafe fn kbts_GlyphIteratorNext(It: *mut glyph_iterator, Glyph: *mut *mut glyph) -> u32;
    /// Ported from `kb_text_shape_procs.odin:274` (inner `kbts_Break`).
    pub unsafe fn kbts_Break(State: *mut break_state, Break: *mut break_type) -> u32;
    /// Ported from `kb_text_shape_procs.odin:288` (inner `kbts_BreakEntireString`).
    pub unsafe fn kbts_BreakEntireString(
        Direction: direction,
        JapaneseLineBreakStyle: japanese_line_break_style,
        ConfigFlags: break_config_flags,
        Input: *mut c_void,
        InputSizeInBytes: c_int,
        InputFormat: text_format,
        Breaks: *mut break_type,
        BreakCapacity: c_int,
        BreakCount: *mut c_int,
        BreakFlags: *mut break_flags,
        BreakFlagCapacity: c_int,
        BreakFlagCount: *mut c_int,
    );
    /// Ported from `kb_text_shape_procs.odin:304` (inner `kbts_BreakEntireStringUtf32`).
    pub unsafe fn kbts_BreakEntireStringUtf32(
        Direction: direction,
        JapaneseLineBreakStyle: japanese_line_break_style,
        ConfigFlags: break_config_flags,
        Utf32: *mut i32,
        Utf32Count: c_int,
        Breaks: *mut break_type,
        BreakCapacity: c_int,
        BreakCount: *mut c_int,
        BreakFlags: *mut break_flags,
        BreakFlagCapacity: c_int,
        BreakFlagCount: *mut c_int,
    );
    /// Ported from `kb_text_shape_procs.odin:319` (inner `kbts_BreakEntireStringUtf8`).
    pub unsafe fn kbts_BreakEntireStringUtf8(
        Direction: direction,
        JapaneseLineBreakStyle: japanese_line_break_style,
        ConfigFlags: break_config_flags,
        Utf8: *const c_char,
        Utf8Length: c_int,
        Breaks: *mut break_type,
        BreakCapacity: c_int,
        BreakCount: *mut c_int,
        BreakFlags: *mut break_flags,
        BreakFlagCapacity: c_int,
        BreakFlagCount: *mut c_int,
    );
    /// Ported from `kb_text_shape_procs.odin:347` (inner `kbts_EncodeUtf8`).
    pub unsafe fn kbts_EncodeUtf8(Codepoint: i32) -> encode_utf8;
    /// Ported from `kb_text_shape_procs.odin:355` (inner `kbts_DecodeUtf8`).
    pub unsafe fn kbts_DecodeUtf8(Utf8: *mut u8, Length: usize) -> decode;
    /// Ported from `kb_text_shape_procs.odin:366` (inner `kbts_GuessTextProperties`).
    pub unsafe fn kbts_GuessTextProperties(
        Text: *mut c_void,
        TextSizeInBytes: c_int,
        Format: text_format,
        Direction: *mut direction,
        Script: *mut script,
    );
    /// Ported from `kb_text_shape_procs.odin:377` (inner `kbts_GuessTextPropertiesUtf32`).
    pub unsafe fn kbts_GuessTextPropertiesUtf32(
        Utf32: *mut i32,
        Utf32Count: c_int,
        Direction: *mut direction,
        Script: *mut script,
    );
    /// Ported from `kb_text_shape_procs.odin:390` (inner `kbts_GuessTextPropertiesUtf8`).
    pub unsafe fn kbts_GuessTextPropertiesUtf8(
        Utf8: *const c_char,
        Utf8Length: c_int,
        Direction: *mut direction,
        Script: *mut script,
    );
}

// --- Odin-level wrappers (slices/strings in, `kbts_*` out) -----------------------

/// Ported from `kb_text_shape_procs.odin:52`
/// (`PlaceShapeContextFixedMemory2`).
///
/// # Safety
///
/// Memory must be valid for reads and writes for the call; the returned context borrows it.
pub unsafe fn PlaceShapeContextFixedMemory2(
    Memory: &[u8],
    Flags: shape_context_flags,
) -> *mut shape_context {
    unsafe {
        kbts_PlaceShapeContextFixedMemory2(
            Memory.as_ptr() as *mut c_void,
            Memory.len() as c_int,
            Flags,
        )
    }
}

/// Ported from `kb_text_shape_procs.odin:60`
/// (`PlaceShapeContextFixedMemory`).
///
/// # Safety
///
/// Memory must be valid for reads and writes for the call; the returned context borrows it.
pub unsafe fn PlaceShapeContextFixedMemory(Memory: &[u8]) -> *mut shape_context {
    unsafe {
        kbts_PlaceShapeContextFixedMemory(Memory.as_ptr() as *mut c_void, Memory.len() as c_int)
    }
}

/// Ported from `kb_text_shape_procs.odin:68`
/// (`ShapePushFontFromMemory`).
///
/// # Safety
///
/// Context must be a live shape context; Memory must be valid to read for the call.
pub unsafe fn ShapePushFontFromMemory(
    Context: *mut shape_context,
    Memory: &[u8],
    FontIndex: c_int,
) -> *mut font {
    unsafe {
        kbts_ShapePushFontFromMemory(
            Context,
            Memory.as_ptr() as *mut c_void,
            Memory.len() as c_int,
            FontIndex,
        )
    }
}

/// Ported from `kb_text_shape_procs.odin:77` (`ShapeRun`).
///
/// # Safety
///
/// Context must be a live shape context that has begun shaping.
pub unsafe fn ShapeRun(Context: *mut shape_context) -> (run, bool) {
    unsafe {
        let mut out: run = core::mem::zeroed();
        let ok = kbts_ShapeRun(Context, &mut out);
        (out, ok != 0)
    }
}

/// Ported from `kb_text_shape_procs.odin:86` (`ShapeUtf32`).
///
/// # Safety
///
/// Context must be a live shape context; Utf32 must be valid to read for the call.
pub unsafe fn ShapeUtf32(Context: *mut shape_context, Utf32: &[i32]) {
    unsafe { kbts_ShapeUtf32(Context, Utf32.as_ptr() as *mut i32, Utf32.len() as c_int) }
}

/// Ported from `kb_text_shape_procs.odin:93` (`ShapeUtf32WithUserId`).
///
/// # Safety
///
/// Context must be a live shape context; Utf32 must be valid to read for the call.
pub unsafe fn ShapeUtf32WithUserId(
    Context: *mut shape_context,
    Utf32: &[i32],
    UserId: c_int,
    UserIdIncrement: c_int,
) {
    unsafe {
        kbts_ShapeUtf32WithUserId(
            Context,
            Utf32.as_ptr() as *mut i32,
            Utf32.len() as c_int,
            UserId,
            UserIdIncrement,
        );
    }
}

/// Ported from `kb_text_shape_procs.odin:101` (`ShapeUtf8`).
///
/// # Safety
///
/// Context must be a live shape context; Utf8 must be valid to read for the call.
pub unsafe fn ShapeUtf8(
    Context: *mut shape_context,
    Utf8: &[u8],
    UserIdGenerationMode: user_id_generation_mode,
) {
    unsafe {
        kbts_ShapeUtf8(
            Context,
            Utf8.as_ptr() as *mut u8,
            Utf8.len() as c_int,
            UserIdGenerationMode,
        );
    }
}

/// Ported from `kb_text_shape_procs.odin:108` (`ShapeUtf8WithUserId`).
///
/// # Safety
///
/// Context must be a live shape context; Utf8 must be valid to read for the call.
pub unsafe fn ShapeUtf8WithUserId(
    Context: *mut shape_context,
    Utf8: &[u8],
    UserId: c_int,
    UserIdGenerationMode: user_id_generation_mode,
) {
    unsafe {
        kbts_ShapeUtf8WithUserId(
            Context,
            Utf8.as_ptr() as *mut u8,
            Utf8.len() as c_int,
            UserId,
            UserIdGenerationMode,
        );
    }
}

/// Ported from `kb_text_shape_procs.odin:125`
/// (`ShapeCodepointIteratorNext`).
///
/// # Safety
///
/// It must be a live codepoint iterator from ShapeCurrentCodepointsIterator.
pub unsafe fn ShapeCodepointIteratorNext(
    It: *mut shape_codepoint_iterator,
) -> (shape_codepoint, c_int, bool) {
    unsafe {
        let mut codepoint: shape_codepoint = core::mem::zeroed();
        let mut index: c_int = 0;
        let ok = kbts_ShapeCodepointIteratorNext(It, &mut codepoint, &mut index);
        (codepoint, index, ok != 0)
    }
}

/// Ported from `kb_text_shape_procs.odin:178` (`LoadFont`).
///
/// # Safety
///
/// Font and State must be valid for writes; FontData must be valid to read for the call.
pub unsafe fn LoadFont(
    Font: *mut font,
    State: *mut load_font_state,
    FontData: &[u8],
    FontIndex: c_int,
) -> (c_int, c_int, load_font_error) {
    unsafe {
        let mut scratch: c_int = 0;
        let mut output: c_int = 0;
        let err = kbts_LoadFont(
            Font,
            State,
            FontData.as_ptr() as *mut c_void,
            FontData.len() as c_int,
            FontIndex,
            &mut scratch,
            &mut output,
        );
        (scratch, output, err)
    }
}

/// Ported from `kb_text_shape_procs.odin:188` (`FontCount`).
///
/// # Safety
///
/// FileData must be valid to read for the call.
pub unsafe fn FontCount(FileData: &[u8]) -> c_int {
    unsafe { kbts_FontCount(FileData.as_ptr() as *mut c_void, FileData.len() as c_int) }
}

/// Ported from `kb_text_shape_procs.odin:197` (`FontFromMemory`).
///
/// # Safety
///
/// FileData must be valid to read for the call; the Allocator pair must match kbts expectations (both null or both set).
pub unsafe fn FontFromMemory(
    FileData: &[u8],
    FontIndex: c_int,
    Allocator: allocator_function,
    AllocatorData: *mut c_void,
) -> font {
    unsafe {
        kbts_FontFromMemory(
            FileData.as_ptr() as *mut c_void,
            FileData.len() as c_int,
            FontIndex,
            Allocator,
            AllocatorData,
        )
    }
}

/// Ported from `kb_text_shape_procs.odin:207` (`ShapeDirect`).
///
/// # Safety
///
/// Scratchpad and Storage must be live; the returned iterator borrows Storage.
pub unsafe fn ShapeDirect(
    Scratchpad: *mut shape_scratchpad,
    Storage: *mut glyph_storage,
    RunDirection: direction,
) -> (glyph_iterator, shape_error) {
    unsafe {
        let mut out: glyph_iterator = core::mem::zeroed();
        let err = kbts_ShapeDirect(Scratchpad, Storage, RunDirection, &mut out);
        (out, err)
    }
}

/// Ported from `kb_text_shape_procs.odin:218` (`SizeOfGlyphConfig`).
///
/// # Safety
///
/// ShapeConfig must be a live shape config; Overrides must be valid to read for the call.
pub unsafe fn SizeOfGlyphConfig(
    ShapeConfig: *mut shape_config,
    Overrides: &[feature_override],
) -> c_int {
    unsafe { kbts_SizeOfGlyphConfig(ShapeConfig, Overrides.as_ptr(), Overrides.len() as c_int) }
}

/// Ported from `kb_text_shape_procs.odin:227` (`PlaceGlyphConfig`).
///
/// # Safety
///
/// ShapeConfig must be a live shape config; Overrides must be valid to read and Memory valid to write for the call.
pub unsafe fn PlaceGlyphConfig(
    ShapeConfig: *mut shape_config,
    Overrides: &[feature_override],
    Memory: *mut c_void,
) -> *mut glyph_config {
    unsafe {
        kbts_PlaceGlyphConfig(
            ShapeConfig,
            Overrides.as_ptr(),
            Overrides.len() as c_int,
            Memory,
        )
    }
}

/// Ported from `kb_text_shape_procs.odin:236` (`CreateGlyphConfig`).
///
/// # Safety
///
/// ShapeConfig must be a live shape config; Overrides must be valid to read for the call.
pub unsafe fn CreateGlyphConfig(
    ShapeConfig: *mut shape_config,
    Overrides: &[feature_override],
    Allocator: allocator_function,
    AllocatorData: *mut c_void,
) -> *mut glyph_config {
    unsafe {
        kbts_CreateGlyphConfig(
            ShapeConfig,
            Overrides.as_ptr(),
            Overrides.len() as c_int,
            Allocator,
            AllocatorData,
        )
    }
}

/// Ported from `kb_text_shape_procs.odin:250` (`GlyphIteratorNext`).
///
/// # Safety
///
/// It must be a live glyph iterator.
pub unsafe fn GlyphIteratorNext(It: *mut glyph_iterator) -> (*mut glyph, bool) {
    unsafe {
        let mut out: *mut glyph = core::ptr::null_mut();
        let ok = kbts_GlyphIteratorNext(It, &mut out);
        (out, ok != 0)
    }
}

/// Ported from `kb_text_shape_procs.odin:271` (`Break`).
///
/// # Safety
///
/// State must be a live break state.
pub unsafe fn Break(State: *mut break_state) -> (break_type, bool) {
    unsafe {
        let mut out: break_type = core::mem::zeroed();
        let ok = kbts_Break(State, &mut out);
        (out, ok != 0)
    }
}

/// Ported from `kb_text_shape_procs.odin:281` (`BreakEntireString`).
///
/// # Safety
///
/// Input, Breaks, BreakCount, BreakFlags and BreakFlagCount must be valid for their reads/writes for the call.
// Argument count mirrors the C entry point exactly.
#[allow(clippy::too_many_arguments)]
pub unsafe fn BreakEntireString(
    Direction: direction,
    JapaneseLineBreakStyle: japanese_line_break_style,
    ConfigFlags: break_config_flags,
    Input: &[u8],
    InputFormat: text_format,
    Breaks: &mut [break_type],
    BreakCount: &mut c_int,
    BreakFlags: &mut [break_flags],
    BreakFlagCount: &mut c_int,
) {
    unsafe {
        kbts_BreakEntireString(
            Direction,
            JapaneseLineBreakStyle,
            ConfigFlags,
            Input.as_ptr() as *mut c_void,
            Input.len() as c_int,
            InputFormat,
            Breaks.as_mut_ptr(),
            Breaks.len() as c_int,
            BreakCount,
            BreakFlags.as_mut_ptr(),
            BreakFlags.len() as c_int,
            BreakFlagCount,
        );
    }
}

/// Ported from `kb_text_shape_procs.odin:296`
/// (`BreakEntireStringUtf32`).
///
/// # Safety
///
/// Utf32, Breaks, BreakCount, BreakFlags and BreakFlagCount must be valid for their reads/writes for the call.
// Argument count mirrors the C entry point exactly.
#[allow(clippy::too_many_arguments)]
pub unsafe fn BreakEntireStringUtf32(
    Direction: direction,
    JapaneseLineBreakStyle: japanese_line_break_style,
    ConfigFlags: break_config_flags,
    Utf32: &[i32],
    Breaks: &mut [break_type],
    BreakCount: &mut c_int,
    BreakFlags: &mut [break_flags],
    BreakFlagCount: &mut c_int,
) {
    unsafe {
        kbts_BreakEntireStringUtf32(
            Direction,
            JapaneseLineBreakStyle,
            ConfigFlags,
            Utf32.as_ptr() as *mut i32,
            Utf32.len() as c_int,
            Breaks.as_mut_ptr(),
            Breaks.len() as c_int,
            BreakCount,
            BreakFlags.as_mut_ptr(),
            BreakFlags.len() as c_int,
            BreakFlagCount,
        );
    }
}

/// Ported from `kb_text_shape_procs.odin:311`
/// (`BreakEntireStringUtf8`).
///
/// # Safety
///
/// Utf8, Breaks, BreakCount, BreakFlags and BreakFlagCount must be valid for their reads/writes for the call.
// Argument count mirrors the C entry point exactly.
#[allow(clippy::too_many_arguments)]
pub unsafe fn BreakEntireStringUtf8(
    Direction: direction,
    JapaneseLineBreakStyle: japanese_line_break_style,
    ConfigFlags: break_config_flags,
    Utf8: &str,
    Breaks: &mut [break_type],
    BreakCount: &mut c_int,
    BreakFlags: &mut [break_flags],
    BreakFlagCount: &mut c_int,
) {
    unsafe {
        kbts_BreakEntireStringUtf8(
            Direction,
            JapaneseLineBreakStyle,
            ConfigFlags,
            Utf8.as_ptr() as *const c_char,
            Utf8.len() as c_int,
            Breaks.as_mut_ptr(),
            Breaks.len() as c_int,
            BreakCount,
            BreakFlags.as_mut_ptr(),
            BreakFlags.len() as c_int,
            BreakFlagCount,
        );
    }
}

/// Ported from `kb_text_shape_procs.odin:341` (`EncodeUtf8`).
///
/// # Safety
///
/// No preconditions; Codepoint should be a valid scalar value.
pub unsafe fn EncodeUtf8(Codepoint: i32) -> ([u8; 4], c_int, bool) {
    unsafe {
        let out = kbts_EncodeUtf8(Codepoint);
        (out.Encoded, out.EncodedLength, out.Valid != 0)
    }
}

/// Ported from `kb_text_shape_procs.odin:351` (`DecodeUtf8`).
///
/// # Safety
///
/// Utf8 must be valid to read for the call.
pub unsafe fn DecodeUtf8(Utf8: &[u8]) -> decode {
    unsafe { kbts_DecodeUtf8(Utf8.as_ptr() as *mut u8, Utf8.len()) }
}

/// Ported from `kb_text_shape_procs.odin:362`
/// (`GuessTextProperties`).
///
/// # Safety
///
/// Text must be valid to read for the call.
pub unsafe fn GuessTextProperties(Text: &[u8], Format: text_format) -> (direction, script) {
    unsafe {
        let mut dir: direction = core::mem::zeroed();
        let mut scr: script = core::mem::zeroed();
        kbts_GuessTextProperties(
            Text.as_ptr() as *mut c_void,
            Text.len() as c_int,
            Format,
            &mut dir,
            &mut scr,
        );
        (dir, scr)
    }
}

/// Ported from `kb_text_shape_procs.odin:374`
/// (`GuessTextPropertiesUtf32`).
///
/// # Safety
///
/// Utf32 must be valid to read for the call.
pub unsafe fn GuessTextPropertiesUtf32(Utf32: &[i32]) -> (direction, script) {
    unsafe {
        let mut dir: direction = core::mem::zeroed();
        let mut scr: script = core::mem::zeroed();
        kbts_GuessTextPropertiesUtf32(
            Utf32.as_ptr() as *mut i32,
            Utf32.len() as c_int,
            &mut dir,
            &mut scr,
        );
        (dir, scr)
    }
}

/// Ported from `kb_text_shape_procs.odin:386`
/// (`GuessTextPropertiesUtf8`).
///
/// # Safety
///
/// Utf8 must be a valid UTF-8 string.
pub unsafe fn GuessTextPropertiesUtf8(Utf8: &str) -> (direction, script) {
    unsafe {
        let mut dir: direction = core::mem::zeroed();
        let mut scr: script = core::mem::zeroed();
        kbts_GuessTextPropertiesUtf8(
            Utf8.as_ptr() as *const c_char,
            Utf8.len() as c_int,
            &mut dir,
            &mut scr,
        );
        (dir, scr)
    }
}

// `AllocatorFromOdinAllocator` (`kb_text_shape_procs.odin:399`) has no Rust
// counterpart by design: it bridges Odin's `runtime.Allocator`, which has
// no meaning in Rust; callers pass an `allocator_function` explicitly.
