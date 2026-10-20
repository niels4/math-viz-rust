//! The frame: NanoVG paint for Clay's layout, layers, overlays and cursor.
//!
//! Ported from `draw.odin`. Every NanoVG call goes through `mv-nvg`'s safe
//! methods. The operations `mv-nvg` cannot express without `unsafe` sit
//! behind the private shims at the bottom of this file, each naming the
//! milestone that owns its real body:
//!
//! * the GL viewport/clear (`gl_target`/`gl_viewport` below, on the
//!   `gl: glow::Context` field `i0` added to [`App`]) and the pixel readback
//!   (`t0c-wire` in `mv-nvg`);
//! * reading Clay's packed unions and render-command storage (`t0c` in
//!   `mv-clay` / `mv-sys`).

use crate::app::App;
use crate::app_loop::Time;
use crate::effects::{Effect, Layer, RING_WIDTH, draw_inset, draw_ring, draw_shadow};
use crate::input::{HitKind, Region, region_of, update_cursor};
use crate::number_box::box_ring_shows;
use crate::plane_area::lay_out_plane;
use crate::settings::settings_line_focus;
use crate::track::track_ring;
use crate::typography::draw_text;
use crate::ui::{after_layout, current_theme, layout};
use crate::widgets::Custom;
use mv_sys::clay;
use mv_theme::Token;
use mv_viewer::Param;

/// Rounds a layout box out to whole backing pixels at the given density.
///
/// Ported from `draw.odin:78` (`snap_box`).
pub fn snap_box(_box: clay::BoundingBox, density: f32) -> clay::BoundingBox {
    fn snap(v: f32, density: f32) -> f32 {
        (v * density + 0.5).floor() / density
    }
    let x0 = snap(_box.x, density);
    let y0 = snap(_box.y, density);
    let x1 = snap(_box.x + _box.width, density);
    let y1 = snap(_box.y + _box.height, density);
    clay::BoundingBox {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    }
}

/// A Clay colour (0 to 255 per channel) as NanoVG takes it (0 to 1).
///
/// Ported from `draw.odin:15` (`nvg_color`).
pub fn nvg_color(c: clay::Color) -> mv_nvg::Color {
    [c[0] / 255.0, c[1] / 255.0, c[2] / 255.0, c[3] / 255.0]
}

/// Corner radii shrunk to fit their box, as CSS takes them: each side's pair
/// scaled down by one factor until it fits (CSS Backgrounds 3 § 5.5), which
/// is also exactly how Clay fits them.
///
/// Ported from `draw.odin:346` (`css_radii`).
pub fn css_radii(_box: clay::BoundingBox, r: clay::CornerRadius) -> clay::CornerRadius {
    let mut f = 1.0;
    let mut fit = |length: f32, sum: f32| {
        if sum > 0.0 && length < sum * f {
            f = length.max(0.0) / sum;
        }
    };
    fit(_box.width, r.topLeft + r.topRight);
    fit(_box.width, r.bottomLeft + r.bottomRight);
    fit(_box.height, r.topLeft + r.bottomLeft);
    fit(_box.height, r.topRight + r.bottomRight);
    clay::CornerRadius {
        topLeft: r.topLeft * f,
        topRight: r.topRight * f,
        bottomLeft: r.bottomLeft * f,
        bottomRight: r.bottomRight * f,
    }
}

/// Each corner radius moved inward by `by`, floored at zero.
///
/// Ported from `draw.odin:361` (`inset_radius`).
pub fn inset_radius(r: clay::CornerRadius, by: f32) -> clay::CornerRadius {
    clay::CornerRadius {
        topLeft: (r.topLeft - by).max(0.0),
        topRight: (r.topRight - by).max(0.0),
        bottomLeft: (r.bottomLeft - by).max(0.0),
        bottomRight: (r.bottomRight - by).max(0.0),
    }
}

/// A theme colour as CSS takes it: partial alpha premultiplies the channels.
///
/// Ported from `draw.odin:27` (`css_background`).
pub fn css_background(c: mv_theme::Color) -> mv_theme::Color {
    if c[3] <= 0.0 || c[3] >= 1.0 {
        return c;
    }
    let a = (c[3] * 255.0).round();
    let mut out = [0.0, 0.0, 0.0, a / 255.0];
    for i in 0..3 {
        out[i] = (c[i] * a).round() / a;
    }
    out
}

/// Emits the box's path with per-corner radii, without painting.
///
/// Callers open the path and fill or stroke it themselves, as the Odin
/// callers do; in particular `draw_ring`/`draw_inset` emit two subpaths
/// into one open path, which a fill here would destroy.
///
/// Ported from `draw.odin:327` (`rounded_rect`).
pub fn rounded_rect(vg: &mut mv_nvg::Context, _box: clay::BoundingBox, radius: clay::CornerRadius) {
    let r = css_radii(_box, radius);
    vg.rounded_rect_varying(
        _box.x,
        _box.y,
        _box.width,
        _box.height,
        r.topLeft,
        r.topRight,
        r.bottomRight,
        r.bottomLeft,
    );
}

/// Fills a plain rectangle.
///
/// Ported from `widgets.odin:395` (`fill_rect`). Deliberate difference from
/// the Odin home: kept here beside `rounded_rect` so its call sites share
/// one import; the body is the Odin body unchanged.
pub fn fill_rect(vg: &mut mv_nvg::Context, r: [f32; 4], ink: mv_nvg::Color) {
    vg.begin_path();
    vg.rect(r[0], r[1], r[2], r[3]);
    vg.fill_color(ink);
    vg.fill();
}

/// Clips NanoVG to the region's clip box, or clears the scissor without one.
///
/// Ported from `draw.odin:319` (`clip_to`).
pub fn clip_to(vg: &mut mv_nvg::Context, r: &Region) {
    if let Some(clip) = r.clip {
        vg.scissor(clip.x, clip.y, clip.width, clip.height);
    } else {
        vg.reset_scissor();
    }
}

/// Paints one frame: the layout, the plane, the overlays and the cursor.
///
/// `commands` is Clay's render-command array over the freshly laid-out
/// frame; the adapter lives here so `draw_commands` keeps its surface
/// signature while the layout hands over an owned `Vec`.
///
/// Ported from `draw.odin:40` (`draw_frame`).
pub fn draw_frame(app: &mut App, now: Time) {
    crate::widgets::clear_customs();
    crate::effects::clear_effects();
    let size = app.size;
    let pixels = app.pixels;
    let density = pixels[0] as f32 / size[0];
    let page = current_theme(app).colors[Token::Background];
    gl_target(app, pixels, [page[0], page[1], page[2], 1.0]);
    app.layers.scaled_count = 0;
    app.vg.begin_frame(size[0], size[1], density);
    let mut commands = settle_layout(app, size);
    lay_out_plane(app);
    let array = clay::RenderCommandArray {
        capacity: commands.len() as i32,
        length: commands.len() as i32,
        internalArray: commands.as_mut_ptr(),
    };
    draw_commands(app, &array);
    draw_overlays(app, now);
    app.vg.end_frame();
    update_cursor(app);
}

/// Paints Clay's render commands: boxes, borders, text, customs, layers.
///
/// Ported from `draw.odin:89` (`draw_commands`).
pub fn draw_commands(app: &mut App, commands: &clay::RenderCommandArray) {
    let density = if app.size[0] > 0.0 {
        app.pixels[0] as f32 / app.size[0]
    } else {
        1.0
    };
    let mut scissor: Option<clay::BoundingBox> = None;
    app.input.texts.clear();
    for i in 0..commands.length {
        let command = command_at(commands, i);
        let mut bb = command.boundingBox;
        app.input.text_clip = scissor;
        app.input.text_z = command.zIndex;
        match command.commandType {
            clay::RenderCommandType::Rectangle | clay::RenderCommandType::Border => {
                bb = snap_box(bb, density);
            }
            _ => {}
        }
        match command.commandType {
            clay::RenderCommandType::Rectangle => {
                let data = rect_of(&command);
                let effect = effect_of(&command);
                if let Some(shadow) = effect.and_then(|e| e.shadow) {
                    draw_shadow(&mut app.vg, bb, data.cornerRadius, shadow);
                }
                app.vg.begin_path();
                rounded_rect(&mut app.vg, bb, data.cornerRadius);
                app.vg
                    .fill_color(css_background(nvg_color(data.backgroundColor)));
                app.vg.fill();
                if let Some(e) = effect.filter(|e| e.inset > 0.0) {
                    draw_inset(&mut app.vg, bb, data.cornerRadius, e.inset, e.inset_ink);
                }
            }
            clay::RenderCommandType::Border => {
                let data = border_of(&command);
                let width = data.width.top as f32;
                let r = inset_radius(data.cornerRadius, width / 2.0);
                let inset = clay::BoundingBox {
                    x: bb.x + width / 2.0,
                    y: bb.y + width / 2.0,
                    width: bb.width - width,
                    height: bb.height - width,
                };
                app.vg.begin_path();
                rounded_rect(&mut app.vg, inset, r);
                app.vg.stroke_color(nvg_color(data.color));
                app.vg.stroke_width(width);
                app.vg.stroke();
            }
            clay::RenderCommandType::Text => {
                draw_text(app, &command);
            }
            clay::RenderCommandType::ScissorStart => {
                app.vg.scissor(bb.x, bb.y, bb.width, bb.height);
                scissor = Some(bb);
                app.input.text_clip = scissor;
            }
            clay::RenderCommandType::ScissorEnd => {
                app.vg.reset_scissor();
                scissor = None;
            }
            clay::RenderCommandType::Custom => {
                let (custom, corners) = custom_of(&command);
                crate::widgets::draw_custom(app, bb, custom, corners);
            }
            clay::RenderCommandType::OverlayColorStart => {
                let Some(mut l) = effect_of(&command).and_then(|e| e.layer) else {
                    continue;
                };
                // The origin is the element's own: its box from Clay, under
                // the `found` guard, as the Odin code reads it.
                let data = element_data(clay::ElementId {
                    id: command.id,
                    offset: 0,
                    baseId: 0,
                    // Odin zero-initialises the rest; the lookup hashes `id`.
                    stringId: clay::String {
                        isStaticallyAllocated: false,
                        length: 0,
                        chars: std::ptr::null_mut(),
                    },
                });
                if data.found {
                    l.origin[0] += data.boundingBox.x;
                    l.origin[1] += data.boundingBox.y;
                    if l.scale != 1.0 && app.layers.scaled_count < SCALED_MAX as i64 {
                        let b = data.boundingBox;
                        app.layers.scaled[app.layers.scaled_count as usize] = clay::BoundingBox {
                            x: l.origin[0] + l.offset[0] + (b.x - l.origin[0]) * l.scale,
                            y: l.origin[1] + l.offset[1] + (b.y - l.origin[1]) * l.scale,
                            width: b.width * l.scale,
                            height: b.height * l.scale,
                        };
                        app.layers.scaled_count += 1;
                    }
                }
                layer_begin(app, l, scissor);
            }
            clay::RenderCommandType::OverlayColorEnd => {
                let Some(_) = effect_of(&command).and_then(|e| e.layer) else {
                    continue;
                };
                layer_end(app, scissor);
            }
            clay::RenderCommandType::None | clay::RenderCommandType::Image => {}
        }
    }
}

/// How many overlay layers can be open at once.
pub const LAYER_DEPTH: usize = 2;

/// The open layers' framebuffers and bookkeeping.
pub struct LayerBuffers {
    /// The open layers' buffers, empty until a layer first needs a slot.
    ///
    /// Deliberate difference from the surface's `[Framebuffer; LAYER_DEPTH]`:
    /// a safe `Framebuffer` has no null state, and `delete` consumes, so a
    /// plain array can neither start empty nor release without a dummy
    /// replacement. `pixels[i]` is the size the slot's buffer was made at;
    /// a `None` slot always pairs with a stale size, so the first
    /// `layer_begin` needing it creates the buffer.
    pub buffers: [Option<mv_nvg::Framebuffer>; LAYER_DEPTH],
    /// The size each buffer was made at.
    pub pixels: [[i32; 2]; LAYER_DEPTH],
    /// The layers open now, the innermost last.
    pub open: [OpenLayer; LAYER_DEPTH],
    pub depth: i64,
    /// Layers nested deeper than `LAYER_DEPTH`, drawn in place.
    pub skipped: i64,
    /// That one was logged, once.
    pub warned: bool,
    /// The boxes this frame's layers composited at a scale other than 1, in
    /// the window's px: the harness writes them beside each shot.
    pub scaled: [clay::BoundingBox; SCALED_MAX],
    pub scaled_count: i64,
}

/// How many scaled boxes a frame records.
pub const SCALED_MAX: usize = 4;

/// One open layer: its config and the GL framebuffer it composites back into.
///
/// `target` is the parent framebuffer's GL name, 0 for the default, read from
/// the driver as the layer begins (draw.odin's `FRAMEBUFFER_BINDING` read):
/// natively the harness draws into its offscreen buffer, whose name is not
/// the default, so an assumed parent rebinds wrong and everything drawn after
/// the layer is lost from the shot. Each level stores the binding it found,
/// so nested layers restore the right parent: with `LAYER_DEPTH` 2 the inner
/// layer's parent is the outer layer's buffer, not the window (the settings
/// panel's swap fade inside the panel's opening layer is the two-deep case).
#[derive(Clone, Copy, Debug)]
pub struct OpenLayer {
    pub layer: Layer,
    pub target: i32,
}

/// Paints the hover ring over the focused element, if the frame wants one.
///
/// Ported from `draw.odin:293` (`draw_overlays`).
pub fn draw_overlays(app: &mut App, _now: Time) {
    let ring_ink = mv_theme::ring(current_theme(app));
    let focus = app.input.focus;
    let show_ring = app.input.focus_visible
        && focus.kind != HitKind::PField
        && !settings_line_focus(focus.kind)
        && (focus.kind != HitKind::Box || box_ring_shows(app, box_param(focus.index)));
    if show_ring {
        let (r, found) = region_of(app, focus);
        if found {
            let (mut bb, mut radius) = (r.r#box, r.radius);
            let mut width = RING_WIDTH;
            if r.hit.kind == HitKind::Track {
                (bb, width) = track_ring(app, r.r#box);
                radius = mv_clay::corner_radius_all(bb.width / 2.0);
            }
            app.vg.save();
            clip_to(&mut app.vg, &r);
            draw_ring(&mut app.vg, bb, radius, width, ring_ink);
            app.vg.restore();
        }
    }
}

/// Frees the layer framebuffers on shutdown.
///
/// Ported from `draw.odin:202` (`release_layers`).
pub fn release_layers(app: &mut App) {
    for (i, slot) in app.layers.buffers.iter_mut().enumerate() {
        if let Some(fb) = slot.take() {
            fb.delete();
        }
        // Keep the None-pairs-with-stale-size invariant: a taken slot must
        // not keep a live size, or the next `layer_begin` would skip creation
        // and read a buffer that is gone.
        app.layers.pixels[i] = [0, 0];
    }
}

/// Begins a NanoVG frame at the window density, restoring the scissor.
///
/// Ported from `draw.odin:282` (`frame_again`).
fn frame_again(app: &mut App, scissor: Option<clay::BoundingBox>) {
    app.vg
        .begin_frame(app.size[0], app.size[1], app.pixels[0] as f32 / app.size[0]);
    if let Some(s) = scissor {
        app.vg.scissor(s.x, s.y, s.width, s.height);
    }
}

/// Redirects painting into the layer's framebuffer, making one if needed.
///
/// `pixels[i]` records the size the slot's buffer was made at, so an
/// unchanged layer keeps its buffer across frames. `target` remembers the GL
/// name of the framebuffer bound as the layer began (0 for the default).
///
/// Ported from `draw.odin:216` (`layer_begin`).
fn layer_begin(app: &mut App, l: Layer, scissor: Option<clay::BoundingBox>) {
    if app.layers.depth == LAYER_DEPTH as i64 {
        if !app.layers.warned {
            eprintln!(
                "function-viewer-rust: a layer nested deeper than {} draws in place, \
                 without its opacity or transform",
                LAYER_DEPTH
            );
            app.layers.warned = true;
        }
        app.layers.skipped += 1;
        return;
    }
    let i = app.layers.depth as usize;
    if app.layers.pixels[i] != app.pixels {
        let fresh = mv_nvg::Framebuffer::create(&mut app.vg, app.pixels[0], app.pixels[1], 0);
        let old = app.layers.buffers[i].replace(fresh);
        if let Some(old) = old {
            old.delete();
        }
        app.layers.pixels[i] = app.pixels;
    }
    app.vg.end_frame();
    // The parent is whatever the driver has bound: the harness's offscreen
    // buffer natively, the default framebuffer on the web (draw.odin reads
    // FRAMEBUFFER_BINDING here for the same reason). Read per level, so a
    // nested layer's parent is the outer layer's buffer.
    let target = {
        use glow::HasContext as _;
        let gl = app.gl.as_ref().expect("GL context: the real app");
        // SAFETY: reading one integer of driver state; no pointers.
        unsafe { gl.get_parameter_i32(glow::FRAMEBUFFER_BINDING) }
    };
    app.layers.open[i] = OpenLayer { layer: l, target };
    app.layers.depth += 1;
    if let Some(fb) = app.layers.buffers[i].as_ref() {
        fb.bind();
    }
    gl_target(app, app.pixels, [0.0, 0.0, 0.0, 0.0]);
    frame_again(app, scissor);
}

/// Composites the layer back into its parent at its opacity and transform.
///
/// Ported from `draw.odin:251` (`layer_end`).
fn layer_end(app: &mut App, scissor: Option<clay::BoundingBox>) {
    if app.layers.skipped > 0 {
        app.layers.skipped -= 1;
        return;
    }
    if app.layers.depth == 0 {
        return;
    }
    app.layers.depth -= 1;
    let i = app.layers.depth as usize;
    let open = app.layers.open[i];
    app.vg.end_frame();
    if open.target == 0 {
        mv_nvg::Framebuffer::bind_default();
    } else {
        // SAFETY: the name came from the driver's own binding while the
        // layer began (above); rebinding it restores the parent target.
        debug_assert!(open.target >= 0, "layer parent binding is a GL name");
        use glow::HasContext as _;
        let gl = app.gl.as_ref().expect("GL context: the real app");
        let name = std::num::NonZeroU32::new(open.target as u32);
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, name.map(glow::NativeFramebuffer));
        }
    }
    // Viewport only, no clear: the parent framebuffer already holds the page
    // drawn so far, and clearing would wipe it before the composite.
    gl_viewport(app, app.pixels);
    frame_again(app, scissor);
    let l = open.layer;
    let w = app.size[0];
    let h = app.size[1];
    app.vg.save();
    app.vg
        .translate(l.origin[0] + l.offset[0], l.origin[1] + l.offset[1]);
    app.vg.scale(l.scale, l.scale);
    app.vg.translate(-l.origin[0], -l.origin[1]);
    let image = layer_image(
        app.layers.buffers[i]
            .as_ref()
            .expect("layer buffer live: layer_begin always fills the slot"),
    );
    let paint = app
        .vg
        .image_pattern(0.0, 0.0, w, h, 0.0, image, l.alpha.clamp(0.0, 1.0));
    app.vg.begin_path();
    app.vg.rect(0.0, 0.0, w, h);
    app.vg.fill_paint(&paint);
    app.vg.fill();
    app.vg.restore();
}

/// Lays out until no element asks to measure text, then returns the commands.
///
/// Ported from `draw.odin:62` (`settle_layout`).
pub fn settle_layout(app: &mut App, size: [f32; 2]) -> Vec<clay::RenderCommand> {
    let mut commands = layout(app, size);
    for _ in 0..2 {
        if !after_layout(app) {
            break;
        }
        commands = layout(app, size);
    }
    commands
}

/// A Box focus's param index as the `Param` it addresses.
///
/// Box focus indices are always parameter ordinals; anything else is a bug.
fn box_param(index: i64) -> Param {
    match index {
        0 => Param::A,
        1 => Param::K,
        2 => Param::B,
        3 => Param::H,
        _ => panic!("function-viewer-rust: Box focus with param index {index}"),
    }
}

/// The GL backing store for `pixels`, cleared to `clear`.
///
/// Ported from `draw.odin:44` (`gl_target`): the viewport over the whole
/// store, then the clear colour and a colour+stencil clear. Runs through the
/// `glow::Context` on `App` (the field `i0` added); the context is current
/// on this thread whenever a frame draws.
pub(crate) fn gl_target(app: &App, pixels: [i32; 2], clear: [f32; 4]) {
    use glow::HasContext as _;
    let gl = app.gl.as_ref().expect("GL context: the real app");
    // SAFETY: the context is current on this thread (made current at start),
    // and the viewport/clear take plain integers and floats.
    unsafe {
        gl.viewport(0, 0, pixels[0], pixels[1]);
        gl.clear_color(clear[0], clear[1], clear[2], clear[3]);
        gl.clear(glow::COLOR_BUFFER_BIT | glow::STENCIL_BUFFER_BIT);
    }
}

/// The GL viewport for `pixels`, without touching the clear state.
///
/// Ported from `draw.odin:265`: `layer_end` composites into the parent's
/// framebuffer, which already holds the page, so it sets the viewport only.
pub(crate) fn gl_viewport(app: &App, pixels: [i32; 2]) {
    use glow::HasContext as _;
    let gl = app.gl.as_ref().expect("GL context: the real app");
    // SAFETY: as in `gl_target`.
    unsafe {
        gl.viewport(0, 0, pixels[0], pixels[1]);
    }
}

/// Clay's layout data for an element id.
///
/// Ported from `draw.odin:154` (the overlay branch's `GetElementData` under
/// the `found` guard): the union and storage reads need `unsafe`, so the
/// safe accessor lives in `mv-clay`. Takes no `App` (see `widgets.rs`'s
/// module docs).
fn element_data(id: clay::ElementId) -> clay::ElementData {
    mv_clay::element_data(id)
}

/// The rectangle payload of a Rectangle command.
///
/// Ported from `draw.odin:105` (`command.renderData.rectangle`): reading
/// `RenderCommand`'s packed union needs `unsafe`, so the safe accessor lives
/// in `mv-clay`. The `draw_commands` match guarantees the command type, so a
/// missing payload is a bug.
fn rect_of(command: &clay::RenderCommand) -> clay::RectangleRenderData {
    mv_clay::rect_of(command).expect("Rectangle command holds a rectangle payload")
}

/// The border payload of a Border command.
///
/// Ported from `draw.odin:120` (`command.renderData.border`): as in
/// `rect_of`, the match guarantees the type.
fn border_of(command: &clay::RenderCommand) -> clay::BorderRenderData {
    mv_clay::border_of(command).expect("Border command holds a border payload")
}

/// The effect Clay's config packed for this command, if any.
///
/// Ported from `draw.odin:106,147` (`(^Effect)(command.userData)`):
/// following the `userData` pointer needs `unsafe`, so the safe copy-out
/// lives in `mv-clay`. The copy keeps no borrow of the frame-lived store
/// (`effects.odin:25` `effect` packs it; `draw_frame` frees the store before
/// the next layout).
fn effect_of(command: &clay::RenderCommand) -> Option<Effect> {
    mv_clay::effect_of::<Effect>(command)
}

/// The custom payload and corner radii of a Custom command.
///
/// Ported from `draw.odin:145` (`(^Custom)(data.customData)`): reading the
/// union and following `customData` both need `unsafe`, so the safe borrow
/// lives in `mv-clay`. The payloads live in the frame-lived thread-local
/// store (`widgets.odin:62` `custom_data` packs a clone; `draw_frame` frees
/// the store before the next layout), so the borrow never outlives the
/// frame.
fn custom_of(command: &clay::RenderCommand) -> (&Custom, clay::CornerRadius) {
    mv_clay::custom_of::<Custom>(command).expect("Custom command holds a Custom payload")
}

/// The `i`th render command of an array.
///
/// Ported from `draw.odin:96` (`RenderCommandArray_Get`): out of range
/// reads the array's default command and fires Clay's error handler (see
/// `mv-clay`'s `command_at` docs); the draw walk only indexes its own
/// laid-out frame.
fn command_at(commands: &clay::RenderCommandArray, i: i32) -> clay::RenderCommand {
    mv_clay::command_at(commands, i)
}

/// The GL name of a layer buffer's texture, for the composite paint.
///
/// Ported from `draw.odin` (the layer composite): reading the C struct needs
/// `unsafe`, so the copy-out lives in `mv-nvg`.
fn layer_image(fb: &mv_nvg::Framebuffer) -> i32 {
    fb.image()
}
