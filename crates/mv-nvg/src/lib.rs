//! Safe NanoVG (`mv-sys::nvg`): context, frames, paths, paints, scissors,
//! fonts, shaped-run drawing, stroke feathering, images, framebuffers.
//!
//! Ported from the Odin app's use of `vendor/nanovg/nanovg.odin` and
//! `vendor/nanovg/gl/gl.odin`. The raw C surface is re-exported item by
//! item below (every name but the opaque `sys::Context`, which this crate's
//! own `Context` shadows on purpose); the safe methods wrap it. Method
//! bodies land in `t0c-a`/`t0c-b`; this surface only fixes the signatures.
//! Every public item names its Odin declaration.
//!
//! `unsafe` lives in `mv-sys` only: all public API here is safe. The
//! `*Scoped` Odin helpers (deferred `EndFrame`/`Restore`/`Fill`/`Stroke`)
//! become guard structs; dropping a guard runs the deferred call.

use core::ffi::{c_char, c_int, c_uchar};

use glow::HasContext as _;
use mv_sys::nvg as sys;

// The raw C surface, re-exported for binding parity: every `sys::nvg` name
// but `Context` (shadowed by the safe `Context` below on purpose).
pub use sys::{
    BlendFactor, CodepointType, Color, Commands, CompositeOperation, CompositeOperationState,
    CreateFlag, CreateFlags, Framebuffer as RawFramebuffer, GlyphPosition, ImageFlag, ImageFlags,
    LineCapType, Matrix, Paint, Path, PathCache, PlacedGlyph, Point, PointFlag, PointFlags,
    ScissorT, Solidity, State, TextRow, Texture, Vertex, Winding, nvgAddFallbackFont,
    nvgAddFallbackFontId, nvgArc, nvgArcTo, nvgBeginFrame, nvgBeginPath, nvgBezierTo,
    nvgBoxGradient, nvgCancelFrame, nvgCircle, nvgColorHex, nvgCreateFont, nvgCreateFontMem,
    nvgCreateGL3, nvgCreateGLES3, nvgCreateImageRGBA, nvgCurrentTransform, nvgDebugDumpPathCache,
    nvgDegToRad, nvgDeleteGL3, nvgDeleteGLES3, nvgDeleteImage, nvgEllipse, nvgEndFrame, nvgFill,
    nvgFillColor, nvgFillPaint, nvgFindFont, nvgFontBlur, nvgFontFace, nvgFontFaceId, nvgFontSize,
    nvgGlobalAlpha, nvgGlobalCompositeBlendFunc, nvgGlobalCompositeBlendFuncSeparate,
    nvgGlobalCompositeOperation, nvgHSL, nvgHSLA, nvgImagePattern, nvgImageSize,
    nvgIntersectScissor, nvgLerpRGBA, nvgLineCap, nvgLineJoin, nvgLineTo, nvgLinearGradient,
    nvgMiterLimit, nvgMoveTo, nvgPathSolidity, nvgPathWinding, nvgQuadTo, nvgRGB, nvgRGBA,
    nvgRadToDeg, nvgRadialGradient, nvgRect, nvgReset, nvgResetFallbackFonts,
    nvgResetFallbackFontsId, nvgResetScissor, nvgResetTransform, nvgRestore, nvgRotate,
    nvgRoundedRect, nvgRoundedRectVarying, nvgSave, nvgScale, nvgScissor, nvgShapeAntiAlias,
    nvgSkewX, nvgSkewY, nvgStroke, nvgStrokeColor, nvgStrokeFeathered, nvgStrokePaint,
    nvgStrokeWidth, nvgText, nvgTextAlign, nvgTextBounds, nvgTextBox, nvgTextBoxBounds,
    nvgTextBreakLines, nvgTextGlyphPositions, nvgTextGlyphs, nvgTextIcon, nvgTextLetterSpacing,
    nvgTextLineHeight, nvgTextMetrics, nvgTransform, nvgTransformIdentity, nvgTransformInverse,
    nvgTransformMultiply, nvgTransformPoint, nvgTransformPremultiply, nvgTransformRotate,
    nvgTransformScale, nvgTransformSkewX, nvgTransformSkewY, nvgTransformTranslate, nvgTranslate,
    nvgUpdateImage, nvgluBindFramebuffer, nvgluCreateFramebuffer, nvgluDeleteFramebuffer,
};

// The null context (test-app): the one `mod` this surface's owner adds while
// t0c owns the rest of this file.
mod null;

// --- Context ------------------------------------------------------------------

/// The NanoVG context: owns the GL-backend context pointer.
///
/// Ported from `nanovg.odin:182` (`Context`) with creation from
/// `gl.odin:1344` (`Create`): `gl3` for native OpenGL 3.3 core, `gles3` for
/// WebGL2 on the web.
pub struct Context {
    raw: *mut sys::Context,
    gl: Option<glow::Context>,
}

impl Drop for Context {
    /// Delete the backend context; a null context owns nothing.
    ///
    /// Ported from `gl.odin:1365` (`Destroy`): the harness's headless tests
    /// hold `Context::null()`, which this skips (NanoVG tolerates a null
    /// delete the same way). The backend follows the target — GL3 natively,
    /// GLES3 on the web — as the constructors do; a context is only ever
    /// dropped where it was created.
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }
        unsafe {
            #[cfg(target_os = "emscripten")]
            sys::nvgDeleteGLES3(self.raw);
            #[cfg(not(target_os = "emscripten"))]
            sys::nvgDeleteGL3(self.raw);
        }
    }
}

impl Context {
    /// Create a context on the native GL3 backend.
    ///
    /// Ported from `gl.odin:1344` (`Create`, non-wasm path).
    pub fn gl3(flags: CreateFlags) -> Self {
        Self {
            raw: unsafe { sys::nvgCreateGL3(flags) },
            gl: None,
        }
    }

    /// Create a context on the GLES3 backend (the web build).
    ///
    /// Ported from `gl.odin:1344` (`Create`, wasm path).
    pub fn gles3(flags: CreateFlags) -> Self {
        Self {
            raw: unsafe { sys::nvgCreateGLES3(flags) },
            gl: None,
        }
    }

    /// Begin a frame.
    ///
    /// Ported from `nanovg.odin:375` (`BeginFrame`).
    pub fn begin_frame(&mut self, window_width: f32, window_height: f32, device_pixel_ratio: f32) {
        unsafe { sys::nvgBeginFrame(self.raw, window_width, window_height, device_pixel_ratio) }
    }

    /// End a frame.
    ///
    /// Ported from `nanovg.odin:413` (`EndFrame`).
    pub fn end_frame(&mut self) {
        unsafe { sys::nvgEndFrame(self.raw) }
    }

    /// Cancel a frame.
    ///
    /// Ported from `nanovg.odin:407` (`CancelFrame`).
    pub fn cancel_frame(&mut self) {
        unsafe { sys::nvgCancelFrame(self.raw) }
    }

    /// Attach the GL functions, loaded through SDL, for `finish`/`read_pixels`.
    pub fn set_gl(&mut self, gl: glow::Context) {
        self.gl = Some(gl);
    }

    fn gl(&self) -> &glow::Context {
        self.gl.as_ref().expect("set_gl first")
    }

    /// Block until the GPU has finished (a `glFinish`).
    ///
    /// Ported from `harness.odin:462` (`gl.Finish()`): through the loaded
    /// `glow` context when `set_gl` ran, else through NanoVG's own
    /// `glFinish` helper (which needs no setup), so the harness's timing
    /// never silently skips the finish.
    pub fn finish(&self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe { gl.finish() };
        } else {
            unsafe { sys::nvgGLFinish() };
        }
    }

    /// Read back RGBA rows, bottom to top as `glReadPixels` gives them.
    pub fn read_pixels(&self, w: i32, h: i32) -> Vec<u8> {
        let mut px = vec![0u8; (w * h * 4) as usize];
        unsafe {
            self.gl().read_pixels(
                0,
                0,
                w,
                h,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut px)),
            );
        }
        px
    }

    /// Begin a frame that ends when the guard drops.
    ///
    /// Ported from `nanovg.odin:396` (`FrameScoped`).
    pub fn frame_scoped(
        &mut self,
        window_width: f32,
        window_height: f32,
        device_pixel_ratio: f32,
    ) -> FrameGuard<'_> {
        self.begin_frame(window_width, window_height, device_pixel_ratio);
        FrameGuard {
            context: &mut *self,
        }
    }

    /// Push state.
    ///
    /// Ported from `nanovg.odin:683` (`Save`).
    pub fn save(&mut self) {
        unsafe { sys::nvgSave(self.raw) }
    }

    /// Pop state.
    ///
    /// Ported from `nanovg.odin:697` (`Restore`).
    pub fn restore(&mut self) {
        unsafe { sys::nvgRestore(self.raw) }
    }

    /// Push state that pops when the guard drops.
    ///
    /// Ported from `nanovg.odin:707` (`SaveScoped`).
    pub fn save_scoped(&mut self) -> SaveGuard<'_> {
        self.save();
        SaveGuard {
            context: &mut *self,
        }
    }

    /// Reset state to the default.
    ///
    /// Ported from `nanovg.odin:721` (`Reset`).
    pub fn reset(&mut self) {
        unsafe { sys::nvgReset(self.raw) }
    }

    /// Toggle shape anti-aliasing.
    ///
    /// Ported from `nanovg.odin:755` (`ShapeAntiAlias`).
    pub fn shape_anti_alias(&mut self, enabled: bool) {
        unsafe { sys::nvgShapeAntiAlias(self.raw, enabled as c_int) }
    }

    /// Set the stroke width.
    ///
    /// Ported from `nanovg.odin:761` (`StrokeWidth`).
    pub fn stroke_width(&mut self, width: f32) {
        unsafe { sys::nvgStrokeWidth(self.raw, width) }
    }

    /// Set the miter limit.
    ///
    /// Ported from `nanovg.odin:768` (`MiterLimit`).
    pub fn miter_limit(&mut self, limit: f32) {
        unsafe { sys::nvgMiterLimit(self.raw, limit) }
    }

    /// Set the line cap.
    ///
    /// Ported from `nanovg.odin:775` (`LineCap`).
    pub fn line_cap(&mut self, cap: LineCapType) {
        unsafe { sys::nvgLineCap(self.raw, cap as c_int) }
    }

    /// Set the line join.
    ///
    /// Ported from `nanovg.odin:782` (`LineJoin`). NanoVG's own design: one
    /// `LineCapType` enum serves both cap and join.
    pub fn line_join(&mut self, join: LineCapType) {
        unsafe { sys::nvgLineJoin(self.raw, join as c_int) }
    }

    /// Set the global alpha.
    ///
    /// Ported from `nanovg.odin:789` (`GlobalAlpha`).
    pub fn global_alpha(&mut self, alpha: f32) {
        unsafe { sys::nvgGlobalAlpha(self.raw, alpha) }
    }

    /// Set the stroke color.
    ///
    /// Ported from `nanovg.odin:795` (`StrokeColor`).
    pub fn stroke_color(&mut self, color: Color) {
        unsafe { sys::nvgStrokeColor(self.raw, color) }
    }

    /// Set the stroke paint.
    ///
    /// Ported from `nanovg.odin:801` (`StrokePaint`).
    pub fn stroke_paint(&mut self, paint: &Paint) {
        unsafe { sys::nvgStrokePaint(self.raw, *paint) }
    }

    /// Set the fill color.
    ///
    /// Ported from `nanovg.odin:808` (`FillColor`).
    pub fn fill_color(&mut self, color: Color) {
        unsafe { sys::nvgFillColor(self.raw, color) }
    }

    /// Set the fill paint.
    ///
    /// Ported from `nanovg.odin:814` (`FillPaint`).
    pub fn fill_paint(&mut self, paint: &Paint) {
        unsafe { sys::nvgFillPaint(self.raw, *paint) }
    }

    /// Set the composite operation.
    ///
    /// Ported from `nanovg.odin:1209` (`GlobalCompositeOperation`).
    pub fn global_composite_operation(&mut self, op: CompositeOperation) {
        unsafe { sys::nvgGlobalCompositeOperation(self.raw, op as c_int) }
    }

    /// Set the composite blend function.
    ///
    /// Ported from `nanovg.odin:1215` (`GlobalCompositeBlendFunc`).
    pub fn global_composite_blend_func(&mut self, sfactor: BlendFactor, dfactor: BlendFactor) {
        unsafe { sys::nvgGlobalCompositeBlendFunc(self.raw, sfactor as c_int, dfactor as c_int) }
    }

    /// Set the separate composite blend function.
    ///
    /// Ported from `nanovg.odin:1220` (`GlobalCompositeBlendFuncSeparate`).
    pub fn global_composite_blend_func_separate(
        &mut self,
        src_rgb: BlendFactor,
        dst_rgb: BlendFactor,
        src_alpha: BlendFactor,
        dst_alpha: BlendFactor,
    ) {
        unsafe {
            sys::nvgGlobalCompositeBlendFuncSeparate(
                self.raw,
                src_rgb as c_int,
                dst_rgb as c_int,
                src_alpha as c_int,
                dst_alpha as c_int,
            )
        }
    }

    /// Apply a transform.
    ///
    /// Ported from `nanovg.odin:838` (`Transform`).
    pub fn transform(&mut self, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) {
        unsafe { sys::nvgTransform(self.raw, a, b, c, d, e, f) }
    }

    /// Reset the transform.
    ///
    /// Ported from `nanovg.odin:844` (`ResetTransform`).
    pub fn reset_transform(&mut self) {
        unsafe { sys::nvgResetTransform(self.raw) }
    }

    /// Translate.
    ///
    /// Ported from `nanovg.odin:850` (`Translate`).
    pub fn translate(&mut self, x: f32, y: f32) {
        unsafe { sys::nvgTranslate(self.raw, x, y) }
    }

    /// Rotate.
    ///
    /// Ported from `nanovg.odin:858` (`Rotate`).
    pub fn rotate(&mut self, angle: f32) {
        unsafe { sys::nvgRotate(self.raw, angle) }
    }

    /// Skew along X.
    ///
    /// Ported from `nanovg.odin:866` (`SkewX`).
    pub fn skew_x(&mut self, angle: f32) {
        unsafe { sys::nvgSkewX(self.raw, angle) }
    }

    /// Skew along Y.
    ///
    /// Ported from `nanovg.odin:874` (`SkewY`).
    pub fn skew_y(&mut self, angle: f32) {
        unsafe { sys::nvgSkewY(self.raw, angle) }
    }

    /// Scale.
    ///
    /// Ported from `nanovg.odin:882` (`Scale`).
    pub fn scale(&mut self, x: f32, y: f32) {
        unsafe { sys::nvgScale(self.raw, x, y) }
    }

    /// Read the current transform.
    ///
    /// Ported from `nanovg.odin:896` (`CurrentTransform`).
    pub fn current_transform(&self) -> Matrix {
        let mut m: Matrix = [0.0; 6];
        unsafe { sys::nvgCurrentTransform(self.raw, m.as_mut_ptr()) };
        m
    }

    /// Create an RGBA image from bytes.
    ///
    /// Ported from `nanovg.odin:915` (`CreateImageRGBA`); a `None` image is
    /// the uninitialized-image path the framebuffer takes.
    pub fn create_image_rgba(
        &mut self,
        w: i32,
        h: i32,
        image_flags: ImageFlags,
        data: Option<&[u8]>,
    ) -> i32 {
        let data_ptr = data.map_or(core::ptr::null(), |d| d.as_ptr());
        unsafe { sys::nvgCreateImageRGBA(self.raw, w, h, image_flags, data_ptr as *const c_uchar) }
    }

    /// Update an image's bytes.
    ///
    /// Ported from `nanovg.odin:927` (`UpdateImage`).
    pub fn update_image(&mut self, image: i32, data: &[u8]) {
        unsafe { sys::nvgUpdateImage(self.raw, image, data.as_ptr() as *const c_uchar) }
    }

    /// An image's dimensions.
    ///
    /// Ported from `nanovg.odin:939` (`ImageSize`).
    pub fn image_size(&self, image: i32) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        unsafe { sys::nvgImageSize(self.raw, image, &mut w, &mut h) };
        (w, h)
    }

    /// Delete an image.
    ///
    /// Ported from `nanovg.odin:946` (`DeleteImage`).
    pub fn delete_image(&mut self, image: i32) {
        unsafe { sys::nvgDeleteImage(self.raw, image) }
    }

    /// A linear gradient paint.
    ///
    /// Ported from `nanovg.odin:963` (`LinearGradient`).
    pub fn linear_gradient(
        &self,
        sx: f32,
        sy: f32,
        ex: f32,
        ey: f32,
        icol: Color,
        ocol: Color,
    ) -> Paint {
        unsafe { sys::nvgLinearGradient(self.raw, sx, sy, ex, ey, icol, ocol) }
    }

    /// A radial gradient paint.
    ///
    /// Ported from `nanovg.odin:1008` (`RadialGradient`).
    pub fn radial_gradient(
        &self,
        cx: f32,
        cy: f32,
        inr: f32,
        outr: f32,
        icol: Color,
        ocol: Color,
    ) -> Paint {
        unsafe { sys::nvgRadialGradient(self.raw, cx, cy, inr, outr, icol, ocol) }
    }

    /// A box gradient paint.
    ///
    /// Ported from `nanovg.odin:1039` (`BoxGradient`).
    // Argument count mirrors the C entry point exactly.
    #[allow(clippy::too_many_arguments)]
    pub fn box_gradient(
        &self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        f: f32,
        icol: Color,
        ocol: Color,
    ) -> Paint {
        unsafe { sys::nvgBoxGradient(self.raw, x, y, w, h, r, f, icol, ocol) }
    }

    /// An image pattern paint.
    ///
    /// Ported from `nanovg.odin:1068` (`ImagePattern`).
    // Argument count mirrors the C entry point exactly.
    #[allow(clippy::too_many_arguments)]
    pub fn image_pattern(
        &self,
        cx: f32,
        cy: f32,
        w: f32,
        h: f32,
        angle: f32,
        image: i32,
        alpha: f32,
    ) -> Paint {
        unsafe { sys::nvgImagePattern(self.raw, cx, cy, w, h, angle, image, alpha) }
    }

    /// Set the scissor rectangle.
    ///
    /// Ported from `nanovg.odin:1098` (`Scissor`).
    pub fn scissor(&mut self, x: f32, y: f32, w: f32, h: f32) {
        unsafe { sys::nvgScissor(self.raw, x, y, w, h) }
    }

    /// Intersect the scissor rectangle.
    ///
    /// Ported from `nanovg.odin:1124` (`IntersectScissor`).
    pub fn intersect_scissor(&mut self, x: f32, y: f32, w: f32, h: f32) {
        unsafe { sys::nvgIntersectScissor(self.raw, x, y, w, h) }
    }

    /// Clear the scissor.
    ///
    /// Ported from `nanovg.odin:1167` (`ResetScissor`).
    pub fn reset_scissor(&mut self) {
        unsafe { sys::nvgResetScissor(self.raw) }
    }

    /// Begin a path.
    ///
    /// Ported from `nanovg.odin:2211` (`BeginPath`).
    pub fn begin_path(&mut self) {
        unsafe { sys::nvgBeginPath(self.raw) }
    }

    /// Begin a path that fills when the guard drops.
    ///
    /// Ported from `nanovg.odin:2217` (`FillScoped`).
    pub fn fill_scoped(&mut self) -> FillGuard<'_> {
        self.begin_path();
        FillGuard {
            context: &mut *self,
        }
    }

    /// Begin a path that strokes when the guard drops.
    ///
    /// Ported from `nanovg.odin:2222` (`StrokeScoped`).
    pub fn stroke_scoped(&mut self) -> StrokeGuard<'_> {
        self.begin_path();
        StrokeGuard {
            context: &mut *self,
        }
    }

    /// Begin a path that fills and strokes when the guard drops.
    ///
    /// Ported from `nanovg.odin:2227` (`FillStrokeScoped`).
    pub fn fill_stroke_scoped(&mut self) -> FillStrokeGuard<'_> {
        self.begin_path();
        FillStrokeGuard {
            context: &mut *self,
        }
    }

    /// Move the pen.
    ///
    /// Ported from `nanovg.odin:2232` (`MoveTo`).
    pub fn move_to(&mut self, x: f32, y: f32) {
        unsafe { sys::nvgMoveTo(self.raw, x, y) }
    }

    /// Add a line.
    ///
    /// Ported from `nanovg.odin:2237` (`LineTo`).
    pub fn line_to(&mut self, x: f32, y: f32) {
        unsafe { sys::nvgLineTo(self.raw, x, y) }
    }

    /// Add a cubic bezier.
    ///
    /// Ported from `nanovg.odin:2242` (`BezierTo`).
    pub fn bezier_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        unsafe { sys::nvgBezierTo(self.raw, c1x, c1y, c2x, c2y, x, y) }
    }

    /// Add a quadratic bezier.
    ///
    /// Ported from `nanovg.odin:2252` (`QuadTo`).
    pub fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        unsafe { sys::nvgQuadTo(self.raw, cx, cy, x, y) }
    }

    /// Add an arc segment.
    ///
    /// Ported from `nanovg.odin:2267` (`ArcTo`).
    pub fn arc_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, radius: f32) {
        unsafe { sys::nvgArcTo(self.raw, x1, y1, x2, y2, radius) }
    }

    /// Close the path.
    ///
    /// Ported from `nanovg.odin:2395` (`ClosePath`).
    pub fn close_path(&mut self) {
        unsafe { sys::nvgClosePath(self.raw) }
    }

    /// Set the path winding.
    ///
    /// Ported from `nanovg.odin:2400` (`PathWinding`).
    pub fn path_winding(&mut self, direction: Winding) {
        unsafe { sys::nvgPathWinding(self.raw, direction as c_int) }
    }

    /// Set the path solidity.
    ///
    /// Ported from `nanovg.odin:2405` (`PathSolidity`).
    pub fn path_solidity(&mut self, solidity: Solidity) {
        unsafe { sys::nvgPathSolidity(self.raw, solidity as c_int) }
    }

    /// Add a rectangle path.
    ///
    /// Ported from `nanovg.odin:2410` (`Rect`).
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        unsafe { sys::nvgRect(self.raw, x, y, w, h) }
    }

    /// Add a rounded rectangle path.
    ///
    /// Ported from `nanovg.odin:2421` (`RoundedRect`).
    pub fn rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32) {
        unsafe { sys::nvgRoundedRect(self.raw, x, y, w, h, radius) }
    }

    /// Add a rounded rectangle with per-corner radii.
    ///
    /// Ported from `nanovg.odin:2426` (`RoundedRectVarying`).
    // Argument count mirrors the C entry point exactly.
    #[allow(clippy::too_many_arguments)]
    pub fn rounded_rect_varying(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rad_top_left: f32,
        rad_top_right: f32,
        rad_bottom_right: f32,
        rad_bottom_left: f32,
    ) {
        unsafe {
            sys::nvgRoundedRectVarying(
                self.raw,
                x,
                y,
                w,
                h,
                rad_top_left,
                rad_top_right,
                rad_bottom_right,
                rad_bottom_left,
            )
        }
    }

    /// Add an ellipse path.
    ///
    /// Ported from `nanovg.odin:2464` (`Ellipse`).
    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32) {
        unsafe { sys::nvgEllipse(self.raw, cx, cy, rx, ry) }
    }

    /// Add a circle path.
    ///
    /// Ported from `nanovg.odin:2476` (`Circle`).
    pub fn circle(&mut self, cx: f32, cy: f32, radius: f32) {
        unsafe { sys::nvgCircle(self.raw, cx, cy, radius) }
    }

    /// Add an arc path.
    ///
    /// Ported from `nanovg.odin:2326` (`Arc`).
    pub fn arc(&mut self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32, dir: Winding) {
        unsafe { sys::nvgArc(self.raw, cx, cy, r, a0, a1, dir as c_int) }
    }

    /// Fill the path.
    ///
    /// Ported from `nanovg.odin:2481` (`Fill`).
    pub fn fill(&mut self) {
        unsafe { sys::nvgFill(self.raw) }
    }

    /// Stroke the path.
    ///
    /// Ported from `nanovg.odin:2516` (`Stroke`).
    pub fn stroke(&mut self) {
        unsafe { sys::nvgStroke(self.raw) }
    }

    /// Stroke with an anti-aliased edge of the given width.
    ///
    /// Ported from `nanovg.odin:2564` (`StrokeFeathered`): the plane glow's
    /// stacked feathered strokes (M6).
    pub fn stroke_feathered(&mut self, feather: f32) {
        unsafe { sys::nvgStrokeFeathered(self.raw, feather) }
    }

    /// Dump the path cache (debug builds).
    ///
    /// Ported from `nanovg.odin:2571` (`DebugDumpPathCache`).
    pub fn debug_dump_path_cache(&self) {
        unsafe { sys::nvgDebugDumpPathCache(self.raw) }
    }

    /// Load a font from memory.
    ///
    /// Ported from `nanovg.odin:2635` (`CreateFontMem`).
    pub fn create_font_mem(&mut self, name: &str, data: &[u8], free_loaded_data: bool) -> i32 {
        let name = std::ffi::CString::new(name).expect("font name must not contain NUL");
        unsafe {
            sys::nvgCreateFontMem(
                self.raw,
                name.as_ptr(),
                data.as_ptr() as *mut c_uchar,
                data.len() as c_int,
                free_loaded_data as c_int,
            )
        }
    }

    /// Find a loaded font by name.
    ///
    /// Ported from `nanovg.odin:2640` (`FindFont`).
    pub fn find_font(&mut self, name: &str) -> i32 {
        let name = std::ffi::CString::new(name).expect("font name must not contain NUL");
        unsafe { sys::nvgFindFont(self.raw, name.as_ptr()) }
    }

    /// Add a fallback font by id.
    ///
    /// Ported from `nanovg.odin:2649` (`AddFallbackFontId`).
    pub fn add_fallback_font_id(&mut self, base_font: i32, fallback_font: i32) -> i32 {
        unsafe { sys::nvgAddFallbackFontId(self.raw, base_font, fallback_font) }
    }

    /// Add a fallback font by name.
    ///
    /// Ported from `nanovg.odin:2658` (`AddFallbackFont`).
    pub fn add_fallback_font(&mut self, base_font: &str, fallback_font: &str) -> i32 {
        let base = std::ffi::CString::new(base_font).expect("font name must not contain NUL");
        let fallback =
            std::ffi::CString::new(fallback_font).expect("font name must not contain NUL");
        unsafe { sys::nvgAddFallbackFont(self.raw, base.as_ptr(), fallback.as_ptr()) }
    }

    /// Clear a font's fallbacks by id.
    ///
    /// Ported from `nanovg.odin:2667` (`ResetFallbackFontsId`).
    pub fn reset_fallback_fonts_id(&mut self, base_font: i32) {
        unsafe { sys::nvgResetFallbackFontsId(self.raw, base_font) }
    }

    /// Clear a font's fallbacks by name.
    ///
    /// Ported from `nanovg.odin:2672` (`ResetFallbackFonts`).
    pub fn reset_fallback_fonts(&mut self, base_font: &str) {
        let base = std::ffi::CString::new(base_font).expect("font name must not contain NUL");
        unsafe { sys::nvgResetFallbackFonts(self.raw, base.as_ptr()) }
    }

    /// Set the font size.
    ///
    /// Ported from `nanovg.odin:2677` (`FontSize`).
    pub fn font_size(&mut self, size: f32) {
        unsafe { sys::nvgFontSize(self.raw, size) }
    }

    /// Set the font blur.
    ///
    /// Ported from `nanovg.odin:2683` (`FontBlur`).
    pub fn font_blur(&mut self, blur: f32) {
        unsafe { sys::nvgFontBlur(self.raw, blur) }
    }

    /// Set the letter spacing.
    ///
    /// Ported from `nanovg.odin:2689` (`TextLetterSpacing`).
    pub fn text_letter_spacing(&mut self, spacing: f32) {
        unsafe { sys::nvgTextLetterSpacing(self.raw, spacing) }
    }

    /// Set the line height.
    ///
    /// Ported from `nanovg.odin:2695` (`TextLineHeight`).
    pub fn text_line_height(&mut self, line_height: f32) {
        unsafe { sys::nvgTextLineHeight(self.raw, line_height) }
    }

    /// Set the text alignment.
    ///
    /// Ported from `nanovg.odin:2713` (`TextAlign`, with `2701`
    /// `TextAlignHorizontal` and `2707` `TextAlignVertical` folded in).
    pub fn text_align(&mut self, horizontal: AlignHorizontal, vertical: AlignVertical) {
        let align = (1 << horizontal as c_int) | (1 << (3 + vertical as c_int));
        unsafe { sys::nvgTextAlign(self.raw, align) }
    }

    /// Select the font by id.
    ///
    /// Ported from `nanovg.odin:2720` (`FontFaceId`).
    pub fn font_face_id(&mut self, font: i32) {
        unsafe { sys::nvgFontFaceId(self.raw, font) }
    }

    /// Select the font by name.
    ///
    /// Ported from `nanovg.odin:2726` (`FontFace`).
    pub fn font_face(&mut self, font: &str) {
        let font = std::ffi::CString::new(font).expect("font name must not contain NUL");
        unsafe { sys::nvgFontFace(self.raw, font.as_ptr()) }
    }

    /// Draw one codepoint at an icon position.
    ///
    /// Ported from `nanovg.odin:2826` (`TextIcon`).
    pub fn text_icon(&mut self, xpos: f32, ypos: f32, codepoint: i32) -> f32 {
        unsafe { sys::nvgTextIcon(self.raw, xpos, ypos, codepoint) }
    }

    /// Draw a string.
    ///
    /// Ported from `nanovg.odin:2907` (`Text`).
    pub fn text(&mut self, x: f32, y: f32, text: &str) -> f32 {
        let range = text.as_bytes().as_ptr_range();
        unsafe {
            sys::nvgText(
                self.raw,
                x,
                y,
                range.start as *const c_char,
                range.end as *const c_char,
            )
        }
    }

    /// Draw a shaped run, glyph by glyph.
    ///
    /// Ported from `nanovg.odin:3006` (`TextGlyphs`): the text chain's draw
    /// path (shaped by kb_text_shape, quarter-pixel pens, baselines on
    /// device pixels).
    pub fn text_glyphs(&mut self, x: f32, y: f32, glyphs: &[PlacedGlyph]) {
        unsafe { sys::nvgTextGlyphs(self.raw, x, y, glyphs.as_ptr(), glyphs.len() as c_int) }
    }

    /// The current style's vertical metrics.
    ///
    /// Ported from `nanovg.odin:3067` (`TextMetrics`).
    pub fn text_metrics(&self) -> (f32, f32, f32) {
        let (mut asc, mut desc, mut lineh) = (0.0, 0.0, 0.0);
        unsafe { sys::nvgTextMetrics(self.raw, &mut asc, &mut desc, &mut lineh) };
        (asc, desc, lineh)
    }

    /// Measure a string, optionally returning its bounds.
    ///
    /// Ported from `nanovg.odin:3095` (`TextBounds`).
    pub fn text_bounds(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        bounds: Option<&mut [f32; 4]>,
    ) -> f32 {
        let bounds_ptr = bounds.map_or(core::ptr::null_mut(), |b| b.as_mut_ptr());
        let range = text.as_bytes().as_ptr_range();
        unsafe {
            sys::nvgTextBounds(
                self.raw,
                x,
                y,
                range.start as *const c_char,
                range.end as *const c_char,
                bounds_ptr,
            )
        }
    }

    /// Draw a wrapped text box.
    ///
    /// Ported from `nanovg.odin:3153` (`TextBox`).
    pub fn text_box(&mut self, x: f32, y: f32, break_row_width: f32, text: &str) {
        let range = text.as_bytes().as_ptr_range();
        unsafe {
            sys::nvgTextBox(
                self.raw,
                x,
                y,
                break_row_width,
                range.start as *const c_char,
                range.end as *const c_char,
            )
        }
    }

    /// Break text into rows.
    ///
    /// Ported from `nanovg.odin:3186` (`TextBreakLines`): the row count the
    /// C entry point returns; the rows themselves land in `rows`.
    pub fn text_break_lines(
        &mut self,
        text: &str,
        break_row_width: f32,
        rows: &mut [TextRow],
    ) -> i32 {
        let range = text.as_bytes().as_ptr_range();
        unsafe {
            sys::nvgTextBreakLines(
                self.raw,
                range.start as *const c_char,
                range.end as *const c_char,
                break_row_width,
                rows.as_mut_ptr(),
                rows.len() as c_int,
            )
        }
    }

    /// The bounds of a wrapped text box.
    ///
    /// Ported from `nanovg.odin:3416` (`TextBoxBounds`).
    pub fn text_box_bounds(
        &mut self,
        x: f32,
        y: f32,
        break_row_width: f32,
        text: &str,
        bounds: &mut [f32; 4],
    ) {
        let range = text.as_bytes().as_ptr_range();
        unsafe {
            sys::nvgTextBoxBounds(
                self.raw,
                x,
                y,
                break_row_width,
                range.start as *const c_char,
                range.end as *const c_char,
                bounds.as_mut_ptr(),
            )
        }
    }

    /// The x positions of a string's glyphs.
    ///
    /// Ported from `nanovg.odin:3497` (`TextGlyphPositions`): the count the
    /// C entry point returns; the positions land in `positions`.
    pub fn text_glyph_positions(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        positions: &mut [GlyphPosition],
    ) -> i32 {
        let range = text.as_bytes().as_ptr_range();
        unsafe {
            sys::nvgTextGlyphPositions(
                self.raw,
                x,
                y,
                range.start as *const c_char,
                range.end as *const c_char,
                positions.as_mut_ptr(),
                positions.len() as c_int,
            )
        }
    }
}

// `Context` owns a raw backend pointer; it stays `!Send`/`!Sync` until the
// body proves the threading story.

// --- Guards -------------------------------------------------------------------

/// Ends the frame on drop.
///
/// Ported from `nanovg.odin:396` (`FrameScoped`).
pub struct FrameGuard<'a> {
    context: &'a mut Context,
}

impl Drop for FrameGuard<'_> {
    fn drop(&mut self) {
        unsafe { sys::nvgEndFrame(self.context.raw) }
    }
}

/// Restores state on drop.
///
/// Ported from `nanovg.odin:707` (`SaveScoped`).
pub struct SaveGuard<'a> {
    context: &'a mut Context,
}

impl Drop for SaveGuard<'_> {
    fn drop(&mut self) {
        unsafe { sys::nvgRestore(self.context.raw) }
    }
}

/// Fills the path on drop.
///
/// Ported from `nanovg.odin:2217` (`FillScoped`).
pub struct FillGuard<'a> {
    context: &'a mut Context,
}

impl Drop for FillGuard<'_> {
    fn drop(&mut self) {
        unsafe { sys::nvgFill(self.context.raw) }
    }
}

/// Strokes the path on drop.
///
/// Ported from `nanovg.odin:2222` (`StrokeScoped`).
pub struct StrokeGuard<'a> {
    context: &'a mut Context,
}

impl Drop for StrokeGuard<'_> {
    fn drop(&mut self) {
        unsafe { sys::nvgStroke(self.context.raw) }
    }
}

/// Fills and strokes the path on drop.
///
/// Ported from `nanovg.odin:2227` (`FillStrokeScoped`).
pub struct FillStrokeGuard<'a> {
    context: &'a mut Context,
}

impl Drop for FillStrokeGuard<'_> {
    fn drop(&mut self) {
        unsafe { sys::nvgFill(self.context.raw) }
        unsafe { sys::nvgStroke(self.context.raw) }
    }
}

// --- Text alignment ---------------------------------------------------------------

/// Horizontal text alignment.
///
/// Ported from `nanovg.odin:13` (`AlignHorizontal`, the fontstash port's
/// type): C packs these into the `nvgTextAlign` bitmask.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlignHorizontal {
    Left,
    Center,
    Right,
}

/// Vertical text alignment.
///
/// Ported from `nanovg.odin:13` (`AlignVertical`, the fontstash port's
/// type): C packs these into the `nvgTextAlign` bitmask.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlignVertical {
    Top,
    Middle,
    Bottom,
    Baseline,
}

// --- Framebuffer ---------------------------------------------------------------------

/// An off-screen render target (`nvglu`).
///
/// Ported from `gl.odin:1388` (`framebuffer`): owns the C
/// `NVGLUframebuffer`, created and deleted through `nvglu*`.
pub struct Framebuffer {
    raw: *mut sys::Framebuffer,
    width: i32,
    height: i32,
}

impl Framebuffer {
    /// Create a framebuffer.
    ///
    /// Ported from `gl.odin:1407` (`CreateFramebuffer`).
    pub fn create(ctx: &mut Context, w: i32, h: i32, image_flags: ImageFlags) -> Self {
        let raw = unsafe { sys::nvgluCreateFramebuffer(ctx.raw, w, h, image_flags) };
        assert!(!raw.is_null());
        Self {
            raw,
            width: w,
            height: h,
        }
    }

    /// Bind as the render target.
    ///
    /// Ported from `gl.odin:1400` (`BindFramebuffer`, non-nil path).
    pub fn bind(&self) {
        unsafe { sys::nvgluBindFramebuffer(self.raw) }
    }

    /// Bind the default framebuffer.
    ///
    /// Ported from `gl.odin:1400` (`BindFramebuffer`, nil path).
    pub fn bind_default() {
        unsafe { sys::nvgluBindFramebuffer(core::ptr::null_mut()) }
    }

    /// Delete the framebuffer.
    ///
    /// Ported from `gl.odin:1452` (`DeleteFramebuffer`).
    pub fn delete(self) {
        unsafe { sys::nvgluDeleteFramebuffer(self.raw) }
    }

    /// The framebuffer's NanoVG image handle, for composite paints.
    ///
    /// The draw walk reads it for its layer buffers (`draw.odin`, the
    /// layer composite); reading the C struct needs `unsafe`, so the
    /// copy-out lives here.
    pub fn image(&self) -> i32 {
        unsafe { (*self.raw).image }
    }

    /// The framebuffer's pixels as RGBA bytes, rows bottom to top as
    /// `glReadPixels` gives them.
    ///
    /// Ported from `harness.odin:585` (`write_shot`'s `gl.ReadPixels` leg):
    /// the direct-GL counterpart to `Context::read_pixels`, which reads the
    /// default framebuffer through the loaded `glow` context. The helper
    /// binds this framebuffer itself; bind the default framebuffer
    /// afterwards.
    pub fn read_pixels(&self) -> Vec<u8> {
        let mut out = vec![0u8; (self.width * self.height * 4) as usize];
        unsafe { sys::nvgluReadPixels(self.raw, self.width, self.height, out.as_mut_ptr()) };
        out
    }
}
