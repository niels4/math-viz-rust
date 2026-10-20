//! NanoVG `ce3bf74` with fontstash, stb_truetype and stb_rect_pack
//! (`vendor/nanovg/nanovg.odin`'s API plus `vendor/nanovg/gl/gl.odin`).
//!
//! The Odin app carries NanoVG as an Odin port of C NanoVG; the Rust port
//! compiles upstream C NanoVG `ce3bf74` instead and re-applies the Odin
//! port's 24 patches in C (`t0c-a`/`t0c-b`, listed in `vendor/README.md`).
//! The `extern` names and signatures below are therefore the C header's
//! (`nanovg.h`, `nanovg_gl.h`, `nanovg_gl_utils.h`), each documented with
//! the Odin proc it answers. Where the Odin port and C differ, C wins and
//! the difference is recorded in `.local/briefs/progress/t0c-surface.md`:
//! - All integer widths are C `int` (`c_int`), even where the Odin port's
//!   `int` is 64-bit (`Paint.image`, font ids, image sizes, `TextRow`).
//! - `BlendFactor`, `ImageFlag`, `Texture` and `PointFlag` discriminants are
//!   the C header's bit values, not the Odin port's 0-based enums.
//! - `TextRow`/`GlyphPosition` carry C string pointers, not the Odin port's
//!   byte offsets; `TextBox`/`TextBounds`/`TextBoxBounds`/
//!   `TextGlyphPositions`/`TextBreakLines` take C `(string, end)` pairs, not
//!   Odin `string`s.
//! - `nvgTextGlyphs`, `nvgStrokeFeathered`, `nvgTextIcon` are the Odin
//!   port's additions (`TextGlyphs`, `StrokeFeathered`, `TextIcon`); `t0c`
//!   defines these symbols in the patched C under exactly these names.
//! - `Params`, `CreateInternal`/`DeleteInternal`, `CreateImageFromHandle`,
//!   `ImageHandle` and the `__`-prefixed helpers are the Odin GL backend's
//!   internals with no C counterpart: no Rust item (recorded exclusions).
//!   Context creation is `nvgCreateGL3`/`nvgCreateGLES3`; framebuffers are
//!   `nvglu*` (`NVGLUframebuffer`, which keeps the C field order including
//!   the leading `ctx` the Odin `framebuffer` also has).
//! - `NO_DELETE` (`1<<6`) is the Odin port's image-flag bit with no C
//!   counterpart yet; `t0c` confirms or renames it with the C patch.

#![allow(non_camel_case_types, non_snake_case)]

use core::ffi::{c_char, c_float, c_int, c_uchar};

// --- Color, transform, vertex --------------------------------------------------

/// Ported from `nanovg.odin:27` (`Color`); C `NVGcolor`.
pub type Color = [c_float; 4];

/// Ported from `nanovg.odin:28` (`Matrix`); C `float[6]`.
pub type Matrix = [c_float; 6];

/// Ported from `nanovg.odin:29` (`Vertex`); C `x, y, u, v`.
pub type Vertex = [c_float; 4];

// --- Tuning constants ---------------------------------------------------------------

/// Ported from `nanovg.odin:16` (`INIT_FONTIMAGE_SIZE`).
pub const INIT_FONTIMAGE_SIZE: c_int = 512;
/// Ported from `nanovg.odin:17` (`MAX_FONTIMAGE_SIZE`).
pub const MAX_FONTIMAGE_SIZE: c_int = 2048;
/// Ported from `nanovg.odin:18` (`MAX_FONTIMAGES`).
pub const MAX_FONTIMAGES: c_int = 4;
/// Ported from `nanovg.odin:20` (`MAX_STATES`).
pub const MAX_STATES: c_int = 32;
/// Ported from `nanovg.odin:21` (`INIT_COMMANDS_SIZE`).
pub const INIT_COMMANDS_SIZE: c_int = 256;
/// Ported from `nanovg.odin:22` (`INIT_POINTS_SIZE`).
pub const INIT_POINTS_SIZE: c_int = 128;
/// Ported from `nanovg.odin:23` (`INIT_PATH_SIZE`).
pub const INIT_PATH_SIZE: c_int = 16;
/// Ported from `nanovg.odin:24` (`INIT_VERTS_SIZE`).
pub const INIT_VERTS_SIZE: c_int = 26;
/// Ported from `nanovg.odin:25` (`KAPPA`).
///
/// Literal kept byte-identical to the C/Odin source; the `f32` rounding is
/// what C's `0.5522847493f` gets.
// The literal's extra digits are the source text, not false precision.
#[allow(clippy::excessive_precision)]
pub const KAPPA: c_float = 0.5522847493;

// --- Flags ---------------------------------------------------------------------

/// Ported from `nanovg.odin:31` (`ImageFlag`); values are C
/// `NVG_IMAGE_*` (`NO_DELETE` is the Odin port's bit, see module docs).
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageFlag {
    GENERATE_MIPMAPS = 1 << 0,
    REPEAT_X = 1 << 1,
    REPEAT_Y = 1 << 2,
    FLIP_Y = 1 << 3,
    PREMULTIPLIED = 1 << 4,
    NEAREST = 1 << 5,
    NO_DELETE = 1 << 6,
}

/// Ported from `nanovg.odin:40` (`ImageFlags`); C passes `int`.
pub type ImageFlags = c_int;

/// Ported from `nanovg.odin:52` (`Winding`); C `NVGwinding`.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Winding {
    CCW = 1,
    CW = 2,
}

/// Ported from `nanovg.odin:57` (`Solidity`); C `NVGsolidity`.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Solidity {
    SOLID = 1,
    HOLE = 2,
}

/// Ported from `nanovg.odin:62` (`LineCapType`); C `NVGlineCap`
/// (one enum serves both cap and join in C as in the Odin port).
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineCapType {
    BUTT = 0,
    ROUND = 1,
    SQUARE = 2,
    BEVEL = 3,
    MITER = 4,
}

/// Ported from `nanovg.odin:70` (`BlendFactor`); values are C
/// `NVG_ZERO` etc., not the Odin port's 0-based enum.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendFactor {
    ZERO = 1 << 0,
    ONE = 1 << 1,
    SRC_COLOR = 1 << 2,
    ONE_MINUS_SRC_COLOR = 1 << 3,
    DST_COLOR = 1 << 4,
    ONE_MINUS_DST_COLOR = 1 << 5,
    SRC_ALPHA = 1 << 6,
    ONE_MINUS_SRC_ALPHA = 1 << 7,
    DST_ALPHA = 1 << 8,
    ONE_MINUS_DST_ALPHA = 1 << 9,
    SRC_ALPHA_SATURATE = 1 << 10,
}

/// Ported from `nanovg.odin:84` (`CompositeOperation`); C values (0-based).
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompositeOperation {
    SOURCE_OVER = 0,
    SOURCE_IN = 1,
    SOURCE_OUT = 2,
    ATOP = 3,
    DESTINATION_OVER = 4,
    DESTINATION_IN = 5,
    DESTINATION_OUT = 6,
    DESTINATION_ATOP = 7,
    LIGHTER = 8,
    COPY = 9,
    XOR = 10,
}

/// Ported from `nanovg.odin:98` (`CompositeOperationState`);
/// C `NVGcompositeOperationState`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeOperationState {
    pub srcRGB: c_int,
    pub dstRGB: c_int,
    pub srcAlpha: c_int,
    pub dstAlpha: c_int,
}

/// Ported from `nanovg.odin:107` (`Texture`); values are C
/// `NVG_TEXTURE_*` (1-based), not the Odin port's 0-based enum.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Texture {
    Alpha = 1,
    RGBA = 2,
}

/// Ported from `nanovg.odin:112` (`ScissorT`); C `NVGscissor`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScissorT {
    pub xform: Matrix,
    pub extent: [c_float; 2],
}

/// Ported from `nanovg.odin:117` (`Commands`); C `NVGcommands`.
/// Path-cache internals; declared for the table, not crossed by value.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Commands {
    MOVE_TO = 0,
    LINE_TO = 1,
    BEZIER_TO = 2,
    CLOSE = 3,
    WINDING = 4,
}

/// Ported from `nanovg.odin:125` (`PointFlag`); values are C
/// `NVG_PT_*`. Path-cache internals; declared for the table.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointFlag {
    CORNER = 0x01,
    LEFT = 0x02,
    BEVEL = 0x04,
    INNER_BEVEL = 0x08,
}

/// Ported from `nanovg.odin:130` (`PointFlags`); C passes `int`.
pub type PointFlags = c_int;

// --- Paints ----------------------------------------------------------------------

/// Ported from `nanovg.odin:42` (`Paint`); C `NVGpaint`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paint {
    pub xform: Matrix,
    pub extent: [c_float; 2],
    pub radius: c_float,
    pub feather: c_float,
    pub innerColor: Color,
    pub outerColor: Color,
    pub image: c_int,
}

// --- Text rows, glyphs, framebuffer -------------------------------------------------

/// Ported from `nanovg.odin:3135` (`Text_Row`).
///
/// Carries C `NVGtextRow` string pointers, not the Odin port's byte
/// offsets (see module docs).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TextRow {
    pub start: *const c_char,
    pub end: *const c_char,
    pub next: *const c_char,
    pub width: c_float,
    pub minx: c_float,
    pub maxx: c_float,
}

/// Ported from `nanovg.odin:3143` (`Codepoint_Type`).
/// Line-break classifier; kept for the table (C inlines it as `int`).
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodepointType {
    Space = 0,
    Newline = 1,
    Char = 2,
    CJK = 3,
}

/// Ported from `nanovg.odin:2995` (`Placed_Glyph`); the Odin port's
/// addition, whose layout `t0c` re-applies in C.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedGlyph {
    pub index: i32,
    pub x: c_float,
    pub y: c_float,
}

/// Ported from `nanovg.odin:3489` (`Glyph_Position`).
///
/// Carries the C `NVGglyphPosition` string pointer, not the Odin port's
/// byte offset (see module docs).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GlyphPosition {
    pub str: *const c_char,
    pub x: c_float,
    pub minx: c_float,
    pub maxx: c_float,
}

/// Ported from `gl.odin:1388` (`framebuffer`); C `NVGLUframebuffer`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Framebuffer {
    pub ctx: *mut Context,
    pub fbo: u32,
    pub rbo: u32,
    pub texture: u32,
    pub image: c_int,
}

// --- Opaque internals ---------------------------------------------------------------

/// Ported from `nanovg.odin:133` (`Point`); lives in `nanovg.c`, opaque here.
#[repr(C)]
#[derive(Debug)]
pub struct Point {
    _private: [u8; 0],
}

/// Ported from `nanovg.odin:141` (`PathCache`); lives in `nanovg.c`, opaque here.
#[repr(C)]
#[derive(Debug)]
pub struct PathCache {
    _private: [u8; 0],
}

/// Ported from `nanovg.odin:148` (`Path`); lives in `nanovg.c`, opaque here.
#[repr(C)]
#[derive(Debug)]
pub struct Path {
    _private: [u8; 0],
}

/// Ported from `nanovg.odin:159` (`State`); lives in `nanovg.c`, opaque here.
#[repr(C)]
#[derive(Debug)]
pub struct State {
    _private: [u8; 0],
}

/// Ported from `nanovg.odin:182` (`Context`); C `NVGcontext`, opaque.
#[repr(C)]
#[derive(Debug)]
pub struct Context {
    _private: [u8; 0],
}

// --- Backend creation flags ------------------------------------------------------------

/// Ported from `gl.odin:20` (`CreateFlag`); values are C
/// `NVG_ANTIALIAS` etc.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CreateFlag {
    ANTI_ALIAS = 1 << 0,
    STENCIL_STROKES = 1 << 1,
    DEBUG = 1 << 2,
}

/// Ported from `gl.odin:29` (`CreateFlags`); C passes `int`.
pub type CreateFlags = c_int;

// --- The C API --------------------------------------------------------------------------

// `improper_ctypes` fires on the by-value `Color` (`[f32; 4]`) params and
// returns. C NanoVG really passes `NVGcolor` by value, and on both of this
// port's targets the shapes agree: AAPCS64 treats C's all-float union as a
// homogeneous aggregate (V registers), exactly like Rust's `[f32; 4]`; on
// wasm32 both go indirect. The spike
// (`spikes/vector-native/stack-a/src/main.rs`) proved the equivalent
// by-value color FFI pixel-identical on this machine's arm64.
#[allow(improper_ctypes)]
unsafe extern "C" {
    // Frames (`nanovg.odin:375-413`).
    /// Ported from `nanovg.odin:375` (`BeginFrame`).
    pub unsafe fn nvgBeginFrame(
        ctx: *mut Context,
        windowWidth: c_float,
        windowHeight: c_float,
        devicePixelRatio: c_float,
    );
    /// Ported from `nanovg.odin:407` (`CancelFrame`).
    pub unsafe fn nvgCancelFrame(ctx: *mut Context);
    /// Ported from `nanovg.odin:413` (`EndFrame`).
    pub unsafe fn nvgEndFrame(ctx: *mut Context);
    /// Ported from `nanovg.odin:2629` (`CreateFont`; no path loading in the
    /// port, kept for the table; memory fonts go through `CreateFontMem`).
    pub unsafe fn nvgCreateFont(
        ctx: *mut Context,
        name: *const c_char,
        filename: *const c_char,
    ) -> c_int;

    // State (`nanovg.odin:683-814`).
    /// Ported from `nanovg.odin:683` (`Save`).
    pub unsafe fn nvgSave(ctx: *mut Context);
    /// Ported from `nanovg.odin:697` (`Restore`).
    pub unsafe fn nvgRestore(ctx: *mut Context);
    /// Ported from `nanovg.odin:721` (`Reset`).
    pub unsafe fn nvgReset(ctx: *mut Context);
    /// Ported from `nanovg.odin:755` (`ShapeAntiAlias`).
    pub unsafe fn nvgShapeAntiAlias(ctx: *mut Context, enabled: c_int);
    /// Ported from `nanovg.odin:761` (`StrokeWidth`).
    pub unsafe fn nvgStrokeWidth(ctx: *mut Context, width: c_float);
    /// Ported from `nanovg.odin:768` (`MiterLimit`).
    pub unsafe fn nvgMiterLimit(ctx: *mut Context, limit: c_float);
    /// Ported from `nanovg.odin:775` (`LineCap`).
    pub unsafe fn nvgLineCap(ctx: *mut Context, cap: c_int);
    /// Ported from `nanovg.odin:782` (`LineJoin`).
    pub unsafe fn nvgLineJoin(ctx: *mut Context, join: c_int);
    /// Ported from `nanovg.odin:789` (`GlobalAlpha`).
    pub unsafe fn nvgGlobalAlpha(ctx: *mut Context, alpha: c_float);
    /// Ported from `nanovg.odin:795` (`StrokeColor`).
    pub unsafe fn nvgStrokeColor(ctx: *mut Context, color: Color);
    /// Ported from `nanovg.odin:801` (`StrokePaint`).
    pub unsafe fn nvgStrokePaint(ctx: *mut Context, paint: Paint);
    /// Ported from `nanovg.odin:808` (`FillColor`).
    pub unsafe fn nvgFillColor(ctx: *mut Context, color: Color);
    /// Ported from `nanovg.odin:814` (`FillPaint`).
    pub unsafe fn nvgFillPaint(ctx: *mut Context, paint: Paint);

    // Composite (`nanovg.odin:1209-1226`).
    /// Ported from `nanovg.odin:1209` (`GlobalCompositeOperation`).
    pub unsafe fn nvgGlobalCompositeOperation(ctx: *mut Context, op: c_int);
    /// Ported from `nanovg.odin:1215` (`GlobalCompositeBlendFunc`).
    pub unsafe fn nvgGlobalCompositeBlendFunc(ctx: *mut Context, sfactor: c_int, dfactor: c_int);
    /// Ported from `nanovg.odin:1220` (`GlobalCompositeBlendFuncSeparate`).
    pub unsafe fn nvgGlobalCompositeBlendFuncSeparate(
        ctx: *mut Context,
        srcRGB: c_int,
        dstRGB: c_int,
        srcAlpha: c_int,
        dstAlpha: c_int,
    );

    // Transforms (`nanovg.odin:838-660`).
    /// Ported from `nanovg.odin:838` (`Transform`).
    pub unsafe fn nvgTransform(
        ctx: *mut Context,
        a: c_float,
        b: c_float,
        c: c_float,
        d: c_float,
        e: c_float,
        f: c_float,
    );
    /// Ported from `nanovg.odin:844` (`ResetTransform`).
    pub unsafe fn nvgResetTransform(ctx: *mut Context);
    /// Ported from `nanovg.odin:850` (`Translate`).
    pub unsafe fn nvgTranslate(ctx: *mut Context, x: c_float, y: c_float);
    /// Ported from `nanovg.odin:858` (`Rotate`).
    pub unsafe fn nvgRotate(ctx: *mut Context, angle: c_float);
    /// Ported from `nanovg.odin:866` (`SkewX`).
    pub unsafe fn nvgSkewX(ctx: *mut Context, angle: c_float);
    /// Ported from `nanovg.odin:874` (`SkewY`).
    pub unsafe fn nvgSkewY(ctx: *mut Context, angle: c_float);
    /// Ported from `nanovg.odin:882` (`Scale`).
    pub unsafe fn nvgScale(ctx: *mut Context, x: c_float, y: c_float);
    /// Ported from `nanovg.odin:896` (`CurrentTransform`).
    pub unsafe fn nvgCurrentTransform(ctx: *mut Context, xform: *mut c_float);
    /// Ported from `nanovg.odin:551` (`TransformIdentity`).
    pub unsafe fn nvgTransformIdentity(t: *mut c_float);
    /// Ported from `nanovg.odin:561` (`TransformTranslate`).
    pub unsafe fn nvgTransformTranslate(t: *mut c_float, tx: c_float, ty: c_float);
    /// Ported from `nanovg.odin:571` (`TransformScale`).
    pub unsafe fn nvgTransformScale(t: *mut c_float, sx: c_float, sy: c_float);
    /// Ported from `nanovg.odin:581` (`TransformRotate`).
    pub unsafe fn nvgTransformRotate(t: *mut c_float, a: c_float);
    /// Ported from `nanovg.odin:593` (`TransformSkewX`).
    pub unsafe fn nvgTransformSkewX(t: *mut c_float, a: c_float);
    /// Ported from `nanovg.odin:603` (`TransformSkewY`).
    pub unsafe fn nvgTransformSkewY(t: *mut c_float, a: c_float);
    /// Ported from `nanovg.odin:613` (`TransformMultiply`).
    pub unsafe fn nvgTransformMultiply(t: *mut c_float, s: *const c_float);
    /// Ported from `nanovg.odin:626` (`TransformPremultiply`).
    pub unsafe fn nvgTransformPremultiply(t: *mut c_float, s: *const c_float);
    /// Ported from `nanovg.odin:634` (`TransformInverse`).
    pub unsafe fn nvgTransformInverse(inv: *mut c_float, t: *const c_float) -> c_int;
    /// Ported from `nanovg.odin:654` (`TransformPoint`).
    pub unsafe fn nvgTransformPoint(
        dx: *mut c_float,
        dy: *mut c_float,
        t: *const c_float,
        sx: c_float,
        sy: c_float,
    );
    /// Ported from `nanovg.odin:665` (`DegToRad`).
    pub unsafe fn nvgDegToRad(deg: c_float) -> c_float;
    /// Ported from `nanovg.odin:669` (`RadToDeg`).
    pub unsafe fn nvgRadToDeg(rad: c_float) -> c_float;

    // Colors (`nanovg.odin:463-534`).
    /// Ported from `nanovg.odin:463` (`RGB`).
    pub unsafe fn nvgRGB(r: c_uchar, g: c_uchar, b: c_uchar) -> Color;
    /// Ported from `nanovg.odin:468` (`RGBA`).
    pub unsafe fn nvgRGBA(r: c_uchar, g: c_uchar, b: c_uchar, a: c_uchar) -> Color;
    /// Ported from `nanovg.odin:477` (`LerpRGBA`).
    pub unsafe fn nvgLerpRGBA(c0: Color, c1: Color, u: c_float) -> Color;
    /// Ported from `nanovg.odin:489` (`HSL`).
    pub unsafe fn nvgHSL(h: c_float, s: c_float, l: c_float) -> Color;
    /// Ported from `nanovg.odin:495` (`HSLA`).
    pub unsafe fn nvgHSLA(h: c_float, s: c_float, l: c_float, a: c_uchar) -> Color;
    /// Ported from `nanovg.odin:534` (`ColorHex`).
    pub unsafe fn nvgColorHex(color: u32) -> Color;

    // Images (`nanovg.odin:915-946`).
    /// Ported from `nanovg.odin:915` (`CreateImageRGBA`).
    pub unsafe fn nvgCreateImageRGBA(
        ctx: *mut Context,
        w: c_int,
        h: c_int,
        imageFlags: c_int,
        data: *const c_uchar,
    ) -> c_int;
    /// Ported from `nanovg.odin:927` (`UpdateImage`).
    pub unsafe fn nvgUpdateImage(ctx: *mut Context, image: c_int, data: *const c_uchar);
    /// Ported from `nanovg.odin:939` (`ImageSize`).
    pub unsafe fn nvgImageSize(ctx: *mut Context, image: c_int, w: *mut c_int, h: *mut c_int);
    /// Ported from `nanovg.odin:946` (`DeleteImage`).
    pub unsafe fn nvgDeleteImage(ctx: *mut Context, image: c_int);

    // Paints (`nanovg.odin:963-1074`).
    /// Ported from `nanovg.odin:963` (`LinearGradient`).
    pub unsafe fn nvgLinearGradient(
        ctx: *mut Context,
        sx: c_float,
        sy: c_float,
        ex: c_float,
        ey: c_float,
        icol: Color,
        ocol: Color,
    ) -> Paint;
    /// Ported from `nanovg.odin:1008` (`RadialGradient`).
    pub unsafe fn nvgRadialGradient(
        ctx: *mut Context,
        cx: c_float,
        cy: c_float,
        inr: c_float,
        outr: c_float,
        icol: Color,
        ocol: Color,
    ) -> Paint;
    /// Ported from `nanovg.odin:1039` (`BoxGradient`).
    pub unsafe fn nvgBoxGradient(
        ctx: *mut Context,
        x: c_float,
        y: c_float,
        w: c_float,
        h: c_float,
        r: c_float,
        f: c_float,
        icol: Color,
        ocol: Color,
    ) -> Paint;
    /// Ported from `nanovg.odin:1068` (`ImagePattern`).
    pub unsafe fn nvgImagePattern(
        ctx: *mut Context,
        cx: c_float,
        cy: c_float,
        w: c_float,
        h: c_float,
        angle: c_float,
        image: c_int,
        alpha: c_float,
    ) -> Paint;

    // Scissors (`nanovg.odin:1098-1167`).
    /// Ported from `nanovg.odin:1098` (`Scissor`).
    pub unsafe fn nvgScissor(ctx: *mut Context, x: c_float, y: c_float, w: c_float, h: c_float);
    /// Ported from `nanovg.odin:1124` (`IntersectScissor`).
    pub unsafe fn nvgIntersectScissor(
        ctx: *mut Context,
        x: c_float,
        y: c_float,
        w: c_float,
        h: c_float,
    );
    /// Ported from `nanovg.odin:1167` (`ResetScissor`).
    pub unsafe fn nvgResetScissor(ctx: *mut Context);

    // Paths (`nanovg.odin:2211-2564`).
    /// Ported from `nanovg.odin:2211` (`BeginPath`).
    pub unsafe fn nvgBeginPath(ctx: *mut Context);
    /// Ported from `nanovg.odin:2232` (`MoveTo`).
    pub unsafe fn nvgMoveTo(ctx: *mut Context, x: c_float, y: c_float);
    /// Ported from `nanovg.odin:2237` (`LineTo`).
    pub unsafe fn nvgLineTo(ctx: *mut Context, x: c_float, y: c_float);
    /// Ported from `nanovg.odin:2242` (`BezierTo`).
    pub unsafe fn nvgBezierTo(
        ctx: *mut Context,
        c1x: c_float,
        c1y: c_float,
        c2x: c_float,
        c2y: c_float,
        x: c_float,
        y: c_float,
    );
    /// Ported from `nanovg.odin:2252` (`QuadTo`).
    pub unsafe fn nvgQuadTo(ctx: *mut Context, cx: c_float, cy: c_float, x: c_float, y: c_float);
    /// Ported from `nanovg.odin:2267` (`ArcTo`).
    pub unsafe fn nvgArcTo(
        ctx: *mut Context,
        x1: c_float,
        y1: c_float,
        x2: c_float,
        y2: c_float,
        radius: c_float,
    );
    /// Ported from `nanovg.odin:2326` (`Arc`).
    pub unsafe fn nvgArc(
        ctx: *mut Context,
        cx: c_float,
        cy: c_float,
        r: c_float,
        a0: c_float,
        a1: c_float,
        dir: c_int,
    );
    /// Ported from `nanovg.odin:2395` (`ClosePath`).
    pub unsafe fn nvgClosePath(ctx: *mut Context);
    /// Ported from `nanovg.odin:2400` (`PathWinding`).
    pub unsafe fn nvgPathWinding(ctx: *mut Context, dir: c_int);
    /// Ported from `nanovg.odin:2405` (`PathSolidity`).
    pub unsafe fn nvgPathSolidity(ctx: *mut Context, solidity: c_int);
    /// Ported from `nanovg.odin:2410` (`Rect`).
    pub unsafe fn nvgRect(ctx: *mut Context, x: c_float, y: c_float, w: c_float, h: c_float);
    /// Ported from `nanovg.odin:2421` (`RoundedRect`).
    pub unsafe fn nvgRoundedRect(
        ctx: *mut Context,
        x: c_float,
        y: c_float,
        w: c_float,
        h: c_float,
        radius: c_float,
    );
    /// Ported from `nanovg.odin:2426` (`RoundedRectVarying`).
    pub unsafe fn nvgRoundedRectVarying(
        ctx: *mut Context,
        x: c_float,
        y: c_float,
        w: c_float,
        h: c_float,
        radTopLeft: c_float,
        radTopRight: c_float,
        radBottomRight: c_float,
        radBottomLeft: c_float,
    );
    /// Ported from `nanovg.odin:2464` (`Ellipse`).
    pub unsafe fn nvgEllipse(ctx: *mut Context, cx: c_float, cy: c_float, rx: c_float, ry: c_float);
    /// Ported from `nanovg.odin:2476` (`Circle`).
    pub unsafe fn nvgCircle(ctx: *mut Context, cx: c_float, cy: c_float, r: c_float);
    /// Ported from `nanovg.odin:2481` (`Fill`).
    pub unsafe fn nvgFill(ctx: *mut Context);
    /// Ported from `nanovg.odin:2516` (`Stroke`).
    pub unsafe fn nvgStroke(ctx: *mut Context);
    /// Ported from `nanovg.odin:2564` (`StrokeFeathered`).
    ///
    /// The Odin port's addition; `t0c` defines this symbol in the patched C.
    pub unsafe fn nvgStrokeFeathered(ctx: *mut Context, feather: c_float);
    /// Ported from `nanovg.odin:2571` (`DebugDumpPathCache`).
    pub unsafe fn nvgDebugDumpPathCache(ctx: *mut Context);

    // Fonts (`nanovg.odin:2635-2731`).
    /// Ported from `nanovg.odin:2635` (`CreateFontMem`).
    pub unsafe fn nvgCreateFontMem(
        ctx: *mut Context,
        name: *const c_char,
        data: *mut c_uchar,
        ndata: c_int,
        freeData: c_int,
    ) -> c_int;
    /// Ported from `nanovg.odin:2640` (`FindFont`).
    pub unsafe fn nvgFindFont(ctx: *mut Context, name: *const c_char) -> c_int;
    /// Ported from `nanovg.odin:2649` (`AddFallbackFontId`).
    pub unsafe fn nvgAddFallbackFontId(
        ctx: *mut Context,
        baseFont: c_int,
        fallbackFont: c_int,
    ) -> c_int;
    /// Ported from `nanovg.odin:2658` (`AddFallbackFont`).
    pub unsafe fn nvgAddFallbackFont(
        ctx: *mut Context,
        baseFont: *const c_char,
        fallbackFont: *const c_char,
    ) -> c_int;
    /// Ported from `nanovg.odin:2667` (`ResetFallbackFontsId`).
    pub unsafe fn nvgResetFallbackFontsId(ctx: *mut Context, baseFont: c_int);
    /// Ported from `nanovg.odin:2672` (`ResetFallbackFonts`).
    pub unsafe fn nvgResetFallbackFonts(ctx: *mut Context, baseFont: *const c_char);
    /// Ported from `nanovg.odin:2677` (`FontSize`).
    pub unsafe fn nvgFontSize(ctx: *mut Context, size: c_float);
    /// Ported from `nanovg.odin:2683` (`FontBlur`).
    pub unsafe fn nvgFontBlur(ctx: *mut Context, blur: c_float);
    /// Ported from `nanovg.odin:2689` (`TextLetterSpacing`).
    pub unsafe fn nvgTextLetterSpacing(ctx: *mut Context, spacing: c_float);
    /// Ported from `nanovg.odin:2695` (`TextLineHeight`).
    pub unsafe fn nvgTextLineHeight(ctx: *mut Context, lineHeight: c_float);
    /// Ported from `nanovg.odin:2701-2713` (`TextAlignHorizontal`,
    /// `TextAlignVertical`, `TextAlign`); C takes one `int` bitmask.
    pub unsafe fn nvgTextAlign(ctx: *mut Context, align: c_int);
    /// Ported from `nanovg.odin:2720` (`FontFaceId`).
    pub unsafe fn nvgFontFaceId(ctx: *mut Context, font: c_int);
    /// Ported from `nanovg.odin:2726` (`FontFace`).
    pub unsafe fn nvgFontFace(ctx: *mut Context, font: *const c_char);

    // Text (`nanovg.odin:2826-3541`).
    /// Ported from `nanovg.odin:2826` (`TextIcon`).
    ///
    /// The Odin port's addition; `t0c` defines this symbol in the patched C.
    pub unsafe fn nvgTextIcon(
        ctx: *mut Context,
        xpos: c_float,
        ypos: c_float,
        codepoint: c_int,
    ) -> c_float;
    /// Ported from `nanovg.odin:2907` (`Text`).
    pub unsafe fn nvgText(
        ctx: *mut Context,
        x: c_float,
        y: c_float,
        string: *const c_char,
        end: *const c_char,
    ) -> c_float;
    /// Ported from `nanovg.odin:3006` (`TextGlyphs`).
    ///
    /// The Odin port's addition; `t0c` defines this symbol in the patched C.
    pub unsafe fn nvgTextGlyphs(
        ctx: *mut Context,
        x: c_float,
        y: c_float,
        glyphs: *const PlacedGlyph,
        count: c_int,
    );
    /// Ported from `nanovg.odin:3067` (`TextMetrics`).
    pub unsafe fn nvgTextMetrics(
        ctx: *mut Context,
        ascender: *mut c_float,
        descender: *mut c_float,
        lineH: *mut c_float,
    );
    /// Ported from `nanovg.odin:3095` (`TextBounds`).
    pub unsafe fn nvgTextBounds(
        ctx: *mut Context,
        x: c_float,
        y: c_float,
        string: *const c_char,
        end: *const c_char,
        bounds: *mut c_float,
    ) -> c_float;
    /// Ported from `nanovg.odin:3153` (`TextBox`).
    pub unsafe fn nvgTextBox(
        ctx: *mut Context,
        x: c_float,
        y: c_float,
        breakRowWidth: c_float,
        string: *const c_char,
        end: *const c_char,
    );
    /// Ported from `nanovg.odin:3186` (`TextBreakLines`).
    pub unsafe fn nvgTextBreakLines(
        ctx: *mut Context,
        string: *const c_char,
        end: *const c_char,
        breakRowWidth: c_float,
        rows: *mut TextRow,
        maxRows: c_int,
    ) -> c_int;
    /// Ported from `nanovg.odin:3416` (`TextBoxBounds`).
    pub unsafe fn nvgTextBoxBounds(
        ctx: *mut Context,
        x: c_float,
        y: c_float,
        breakRowWidth: c_float,
        string: *const c_char,
        end: *const c_char,
        bounds: *mut c_float,
    );
    /// Ported from `nanovg.odin:3497` (`TextGlyphPositions`).
    pub unsafe fn nvgTextGlyphPositions(
        ctx: *mut Context,
        x: c_float,
        y: c_float,
        string: *const c_char,
        end: *const c_char,
        positions: *mut GlyphPosition,
        maxPositions: c_int,
    ) -> c_int;

    // Backend creation (`gl.odin:1344-1367`); C `nanovg_gl.h`.
    /// Ported from `gl.odin:1344` (`Create`, GL3 path).
    pub unsafe fn nvgCreateGL3(flags: c_int) -> *mut Context;
    /// Ported from `gl.odin:1365` (`Destroy`, GL3 path).
    pub unsafe fn nvgDeleteGL3(ctx: *mut Context);
    /// Ported from `gl.odin:1344` (`Create`, GLES3 path).
    pub unsafe fn nvgCreateGLES3(flags: c_int) -> *mut Context;
    /// Finishes the GL command stream (for frame readback).
    ///
    /// Ported from `vendor/gl/gl.odin:128` (`Finish`): the helper in
    /// `csrc/nvg_gl_impl.c` calls the real `glFinish` directly, so the
    /// harness needs no GL loader of its own.
    pub unsafe fn nvgGLFinish();
    /// Ported from `gl.odin:1365` (`Destroy`, GLES3 path).
    pub unsafe fn nvgDeleteGLES3(ctx: *mut Context);

    // Framebuffers (`gl.odin:1400-1452`); C `nanovg_gl_utils.h`.
    /// Ported from `gl.odin:1400` (`BindFramebuffer`).
    pub unsafe fn nvgluBindFramebuffer(fb: *mut Framebuffer);
    /// Ported from `gl.odin:1407` (`CreateFramebuffer`).
    pub unsafe fn nvgluCreateFramebuffer(
        ctx: *mut Context,
        w: c_int,
        h: c_int,
        imageFlags: c_int,
    ) -> *mut Framebuffer;
    /// Ported from `gl.odin:1452` (`DeleteFramebuffer`).
    pub unsafe fn nvgluDeleteFramebuffer(fb: *mut Framebuffer);
    /// Reads a framebuffer's pixels into RGBA bytes, rows bottom to top.
    ///
    /// Ported from `vendor/gl/gl.odin:150` (`ReadPixels`): the helper in
    /// `csrc/nvg_gl_impl.c` binds the framebuffer and reads it, as
    /// `harness.odin:585` and `icons.odin:176` do through the GL binding.
    pub unsafe fn nvgluReadPixels(fb: *mut Framebuffer, w: c_int, h: c_int, out: *mut c_uchar);
}
