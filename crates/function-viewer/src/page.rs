//! The page: the top bar over the sidebar and the plane, as `FunctionViewer`
//! mounts it.
//!
//! Ported from `FunctionViewer/FunctionViewer.tsx` and
//! `FunctionViewer.module.css` at `3560eb2`: the page grid (16 px padding,
//! 16 px gaps, title row over content row, 400 px sidebar column), the top
//! bar spanning it, the sidebar column with the Function card (h3's
//! `sidebar.rs`), and the plane's canvas. The frame paint walks Clay's
//! commands through NanoVG; the theme cross-fade paints the last stable
//! frame's snapshot over the new one.

use mv_sys::clay;

use crate::app::App;
use crate::input::{Cursor, Hit, HitKind};
use crate::transition::Transition;

/// The page grid: 16 px padding and gaps, 400 px sidebar column.
pub const PAGE_PAD: f32 = 16.0;
pub const PAGE_GAP: f32 = 16.0;
pub const SIDEBAR_WIDTH: f32 = 400.0;

/// The sidebar's radius (`--radius-card`).
pub const SIDEBAR_RADIUS: f32 = 4.0;

/// The page's sidebar box, for the settings menu's later use and tests.
pub const SIDEBAR_ID: &str = "Sidebar";

/// The plane's two handles: the shift diamond at the live h, k, then the
/// scale square at the unit point of the shown transform, in that order so
/// the square wins a tie (`useTransformHandles.ts` at `96f1e84`: the shift
/// handle's point is the live `{ h, k }`, the scale handle's is
/// `unitPoint(..., shown)`). During a reset the diamond already sits at the
/// boxes' values while the square still rides the spring. The draw and the
/// input dispatch share this one builder, so a press hit-tests what the
/// frame paints.
#[must_use]
pub fn build_handles(app: &App) -> Vec<crate::plot::PlotHandle> {
    let live = crate::flip::live_transform(app);
    let shown = crate::flip::shown_transform(&app.flip, live);
    let slug = *app.func_param.get();
    let unit = crate::functions::unit_x(slug);
    let base = crate::functions::base(slug);
    vec![
        crate::plot::PlotHandle {
            kind: crate::plot::HandleKind::Diamond,
            at: crate::plot::PlotPoint {
                x: live.h,
                y: live.k,
            },
        },
        crate::plot::PlotHandle {
            kind: crate::plot::HandleKind::Square,
            at: crate::transform_curves::unit_point(base, unit, shown),
        },
    ]
}

/// The plane's point markers (`Plot`'s `markers` at `96f1e84`): P always,
/// at `(p, f(p))` with `f` the shown transform applied to the base function
/// (so P rides the flip and reset springs), inked `point_p` with its
/// `"P"` label and drop lines while the slider drags it; Q when the pointer
/// names a point, a ring inked `point_q` with its `"Q"` label, drop lines
/// and the guide always.
#[must_use]
pub fn build_markers(app: &App) -> Vec<crate::plane_geometry::PlotMarker> {
    let live = crate::flip::live_transform(app);
    let shown = crate::flip::shown_transform(&app.flip, live);
    let slug = *app.func_param.get();
    let base = crate::functions::base(slug);
    let f = crate::transform_curves::TransformedFunction {
        base,
        transform: shown,
    };
    let p = *app.p_param.get();
    let point_label = |name: &str, x: f64, y: f64| crate::plane_geometry::PlotLabel {
        name: name.to_string(),
        coords: format!(
            "({}, {})",
            crate::point_format::format_number(x),
            crate::point_format::format_number(y)
        ),
    };
    let p_point = crate::plot::PlotPoint {
        x: p,
        y: f.value(p),
    };
    let mut markers = vec![crate::plane_geometry::PlotMarker {
        point: p_point,
        ink: mv_theme::Token::PointP,
        shape: crate::plane_geometry::MarkerShape::Bullseye,
        label: Some(point_label("P", p_point.x, p_point.y)),
        drop_lines: app.state.p_dragging,
        guide: false,
    }];
    if let Some(q_x) = app.state.q_x {
        let q_point = crate::plot::PlotPoint {
            x: q_x,
            y: f.value(q_x),
        };
        markers.push(crate::plane_geometry::PlotMarker {
            point: q_point,
            ink: mv_theme::Token::PointQ,
            shape: crate::plane_geometry::MarkerShape::Ring,
            label: Some(point_label("Q", q_point.x, q_point.y)),
            drop_lines: true,
            guide: true,
        });
    }
    markers
}

/// Lays out the page in `app`'s Clay context and returns the frame's render
/// commands. The caller opens the layout first and ends it after. Branches
/// on the route (h10-page): the welcome page lays out only its own elements
/// (no sidebar, no plane region, no viewer hit kinds), the viewer as today.
pub fn layout_page(app: &mut App) -> Vec<clay::RenderCommand> {
    if matches!(
        crate::welcome::Route::of(&app.url),
        crate::welcome::Route::Welcome
    ) {
        return crate::welcome::layout_welcome(app, app.size, 1.0);
    }
    app.clay
        .as_mut()
        .expect("clay context initialised")
        .set_layout_dimensions(clay::Dimensions {
            width: app.size[0],
            height: app.size[1],
        });
    app.clay
        .as_mut()
        .expect("clay context initialised")
        .begin_layout();
    let mut root = crate::ui::empty_decl();
    root.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(app.size[0]),
        height: mv_clay::sizing_fixed(app.size[1]),
    };
    root.layout.padding = clay::Padding {
        left: PAGE_PAD as u16,
        right: PAGE_PAD as u16,
        top: PAGE_PAD as u16,
        bottom: PAGE_PAD as u16,
    };
    root.layout.childGap = PAGE_GAP as u16;
    root.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    crate::ui::ui_scope(app, mv_clay::id("Page", 0), &root, |app| {
        crate::top_bar::top_bar(app);
        // The content row: the stub sidebar and the empty body.
        let mut row = crate::ui::empty_decl();
        row.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        };
        row.layout.childGap = PAGE_GAP as u16;
        crate::ui::ui_scope(app, mv_clay::id("ContentRow", 0), &row, |app| {
            let mut side = crate::ui::empty_decl();
            side.layout.sizing = clay::Sizing {
                width: mv_clay::sizing_fixed(SIDEBAR_WIDTH),
                height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
            };
            // `--space-inset` (12) inside the 1 px border at the h4 hash
            // (`--space-card` before): Clay's border takes no layout space,
            // the browser's `border-box` does.
            side.layout.padding = clay::Padding {
                left: (crate::sidebar::SIDE_PAD + 1.0) as u16,
                right: (crate::sidebar::SIDE_PAD + 1.0) as u16,
                top: (crate::sidebar::SIDE_PAD + 1.0) as u16,
                bottom: (crate::sidebar::SIDE_PAD + 1.0) as u16,
            };
            side.backgroundColor = crate::ui::ink(
                crate::ui::current_theme(app),
                mv_theme::Token::BackgroundMuted,
            );
            side.border = clay::BorderElementConfig {
                color: crate::ui::ink(crate::ui::current_theme(app), mv_theme::Token::Border),
                width: mv_clay::border_outside(1),
            };
            side.cornerRadius = crate::ui::corner_radius_all(SIDEBAR_RADIUS);
            crate::ui::ui_scope(app, mv_clay::id(SIDEBAR_ID, 0), &side, |app| {
                // The sidebar's cards (h3's Function card; h4's Transform
                // card joins it here).
                let current = *app.func_param.get();
                crate::sidebar::sidebar(app, current);
            });
            let mut body = crate::ui::empty_decl();
            body.layout.sizing = clay::Sizing {
                width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
                height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
            };
            // The plane's canvas (h2-plane's slot): the border box fills the
            // cell, as the canvas fills its wrapper; the 1 px border and 4 px
            // radius are the CSS values, and the grid paints inside them.
            body.border = clay::BorderElementConfig {
                color: crate::ui::ink(crate::ui::current_theme(app), mv_theme::Token::Border),
                width: mv_clay::border_outside(1),
            };
            body.cornerRadius = crate::ui::corner_radius_all(crate::plane::CANVAS_RADIUS);
            crate::ui::ui_scope(app, mv_clay::id(crate::plane::PLANE_ID, 0), &body, |_| {});
        });
        // The settings menu floats under the gear, above the page: its
        // regions ride one layer up, so the plane's canvas region (declared
        // after the layout, over the same pixels) never shadows the cells
        // and rows it floats over. The gear keeps the page's layer: nothing
        // overlaps it.
        app.input.z = 1;
        crate::settings::menu(app);
        app.input.z = 0;
        // The scrub ruler floats under its row while a box scrubs (h5-ruler).
        crate::ruler::layout_open_drag(app);
    });
    app.clay
        .as_mut()
        .expect("clay context initialised")
        .end_layout()
        .to_vec()
}

/// The canvas's border box, once laid out.
fn plane_box(app: &App) -> Option<clay::BoundingBox> {
    let data = app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(mv_clay::id(crate::plane::PLANE_ID, 0));
    data.found.then_some(data.boundingBox)
}

/// Declares the overlay button's kept region: the pointer shows over it, the
/// click lands on `ReturnToCenter`, and Tab stops on it after the gear. Runs
/// after the layout (the button's box derives from the canvas's), before the
/// regions resolve.
fn declare_plane_button(app: &mut App) {
    if !crate::overlay::button_visible(app.camera.center, app.camera.scale) {
        return;
    }
    let Some(canvas) = plane_box(app) else {
        return;
    };
    let button = crate::overlay::overlay_button_box(&canvas);
    crate::input::region_kept(
        app,
        Hit {
            kind: HitKind::ReturnToCenter,
            index: 0,
        },
        button,
        Cursor::Pointer,
        Cursor::Pointer,
        true,
        crate::ui::corner_radius_all(crate::overlay::BUTTON_RADIUS),
    );
}

/// Paints the plane's slot over the canvas: the grid, axes, labels and scale
/// legend on the content box, then the overlay's brackets and button. Runs
/// after the Clay commands (over the page background), before the
/// transition's fade.
fn draw_plane(app: &mut App, theme: &mv_theme::Theme, now: crate::app_loop::Time) {
    let Some(canvas) = plane_box(app) else {
        return;
    };
    let content = crate::plane::canvas_content(&canvas);
    // The live camera (h2-input's wiring publishes every gesture into it
    // before the draw, so this draws what the frame stepped to — the hook's
    // `cur` at publish, which the React draw reads as props).
    let (center, scale) = (app.camera.center, app.camera.scale);
    let colors = crate::plane::plane_colors(theme);
    // The plotted curves under the shown transform (h6-curve's `Plot`): the
    // switch's curves (`plot.rs`) through `transform_curves`, the dashed
    // original behind each transformed one; the flip's spring blends the
    // shown values. At rest with no values the curves pass through whole.
    // The handles and lines (h6-handles) ride the same shown transform
    // (`transformHandles(..., shown)` and `transformLines(lit, ..., shown)`
    // at `5cef052`). At h7-p-state `transformHandles` is gone (the plane
    // builds its own handles at `251c080`, h7-plane's work), so the draw
    // builds the two handles from the same points here; `transform_lines`
    // takes the lit list with the base function.
    let live = crate::flip::live_transform(app);
    let shown = crate::flip::shown_transform(&app.flip, live);
    let switch = crate::plot::curves(&app.shown);
    let curves = crate::transform_curves::transform_curves(&switch, shown);
    let slug = *app.func_param.get();
    let unit = crate::functions::unit_x(slug);
    let base = crate::functions::base(slug);
    let handles = build_handles(app);
    let lines = crate::transform_curves::transform_lines(&app.state.lit(), base, unit, shown);
    let markers = build_markers(app);
    let mcolors = crate::plane_markers::marker_colors(theme);
    // Natively fonts are always loaded, so the plane always shows its text;
    // the pre-font grid-only state is recorded, not reproduced.
    crate::plane::paint_plane(
        &mut app.fonts,
        &mut app.vg,
        &content,
        center,
        scale,
        &colors,
        true,
        &curves,
        theme.colors[mv_theme::Token::ChartLine],
        theme.colors[mv_theme::Token::ForegroundMuted],
        &handles,
        &lines,
        theme.colors[mv_theme::Token::Primary],
        &markers,
        &mcolors,
    );
    let overlay = crate::overlay::overlay_box(&canvas);
    let ocolors = crate::overlay::overlay_colors(theme);
    crate::overlay::paint_corners(
        &mut app.vg,
        &crate::overlay::corner_boxes(&overlay),
        ocolors.chrome,
    );
    // The button and its tip show off the default camera only (`moved`).
    if !crate::overlay::button_visible(app.camera.center, app.camera.scale) {
        app.overlay_tip_since = None;
        app.timers[crate::app_loop::Timer::OverlayTip] = 0;
        return;
    }
    let button = crate::overlay::overlay_button_box(&canvas);
    let hovered = app.input.hover.kind == HitKind::ReturnToCenter;
    let focused = app.input.focus.kind == HitKind::ReturnToCenter;
    crate::overlay::paint_button(&mut app.vg, &button, &ocolors, hovered, focused);
    // The tooltip's hover clock: armed while the pointer is on the button,
    // dropped the moment it leaves (the CSS `:hover` rule). The timer keeps
    // the app's turns coming until the delay is over, as the scrub rest
    // tip's does.
    if hovered {
        if app.overlay_tip_since.is_none() {
            app.overlay_tip_since = Some(now);
            app.timers[crate::app_loop::Timer::OverlayTip] =
                now + crate::overlay::TIP_DELAY_MS * crate::app_loop::MS;
        }
    } else {
        app.overlay_tip_since = None;
        app.timers[crate::app_loop::Timer::OverlayTip] = 0;
    }
    if crate::overlay::tip_visible(app.overlay_tip_since, focused, app.input.focus_visible, now) {
        crate::overlay::paint_tip(&mut app.fonts, &mut app.vg, &button, &ocolors);
    }
}

/// Paints one frame: the page background, the layout, and the transition's
/// fade when one is in flight. Branches on the route (h10-page): the welcome
/// page paints only its own elements, the viewer as today; the fade and the
/// frame's end are shared.
pub fn draw_page(app: &mut App, now: crate::app_loop::Time) {
    crate::icons::clear_customs();
    let size = app.size;
    let pixels = app.pixels;
    let density = if size[0] > 0.0 {
        pixels[0] as f32 / size[0]
    } else {
        1.0
    };
    let theme = crate::ui::current_theme(app).clone();
    app.vg.begin_frame(size[0], size[1], density);
    // The body's background, over the shell's clear.
    let bg = crate::ui::ink(&theme, mv_theme::Token::Background);
    crate::draw::fill_rect(
        &mut app.vg,
        [0.0, 0.0, size[0], size[1]],
        crate::draw::nvg_color(bg),
    );
    let fading = app
        .transition
        .active
        .map(|active| (active, Transition::progress(active, now)));
    match crate::welcome::Route::of(&app.url) {
        crate::welcome::Route::Welcome => draw_welcome(app, &theme, density),
        crate::welcome::Route::FunctionViewer => draw_viewer(app, &theme, now, density),
    }
    if let Some((active, progress)) = fading {
        paint_transition(app, active, progress, now);
    }
    app.vg.end_frame();
    if fading.is_none() {
        // After the flush: `read_pixels` inside the frame would read the
        // unflushed (stale) backbuffer, so the fade's snapshot waits here.
        snapshot_frame(app);
    }
    crate::input::update_cursor(app);
}

/// Paints the welcome page's frame: the backdrop h10-hero fills in, the
/// still in its frame, the layout's boxes, the menu's chrome and the button's
/// focus ring. The pass keeps the port's premultiplied blend (h10-page's
/// finding: NanoVG's GL backend premultiplies the paint's rgb by its alpha
/// when uploading the uniforms, so straight theme inks composite exactly as
/// the browser does — including the title's alpha ink; only a CPU-side
/// `css_background` would apply twice, and the welcome pass paints no
/// translucent Clay rect).
fn draw_welcome(app: &mut App, theme: &mv_theme::Theme, density: f32) {
    crate::welcome::paint_backdrop(app);
    let commands = crate::welcome::layout_welcome(app, app.size, 1.0);
    crate::input::collect_regions(app);
    // The still in its frame first: the frame's opaque fill sits under the
    // ink in the same pass, so the menu's background (in the commands after)
    // covers the still where the popover overlaps it.
    if let Some(b) = element_box(app, mv_clay::id("WelcomeHeroPlane", 0)) {
        crate::welcome::paint_hero(app, theme, &b, density);
    }
    paint_menu_shadow(app, theme);
    draw_commands(app, density, &commands);
    // The menu's swatches, then the gear's hover border and focus ring.
    crate::settings::paint_swatches(app);
    crate::settings::paint_chrome(app);
    paint_cta_ring(app);
}

/// Paints the welcome button's focus ring: the UA ring on an anchor,
/// measured on ref-h10 (1 px at a 1 px offset), while the link holds
/// keyboard focus.
fn paint_cta_ring(app: &mut App) {
    if app.input.focus.kind != HitKind::WelcomeCta || !app.input.focus_visible {
        return;
    }
    let Some(b) = element_box(app, mv_clay::id("WelcomeCta", 0)) else {
        return;
    };
    let density = if app.size[0] > 0.0 {
        app.pixels[0] as f32 / app.size[0]
    } else {
        1.0
    };
    // The 1 px outline band from the offset outward: a centred stroke on the
    // box grown by offset plus half the width.
    let grown = clay::BoundingBox {
        x: b.x - crate::welcome::CTA_RING_OFFSET - crate::welcome::CTA_RING_WIDTH / 2.0,
        y: b.y - crate::welcome::CTA_RING_OFFSET - crate::welcome::CTA_RING_WIDTH / 2.0,
        width: b.width
            + 2.0 * (crate::welcome::CTA_RING_OFFSET + crate::welcome::CTA_RING_WIDTH / 2.0),
        height: b.height
            + 2.0 * (crate::welcome::CTA_RING_OFFSET + crate::welcome::CTA_RING_WIDTH / 2.0),
    };
    app.vg.begin_path();
    crate::draw::rounded_rect(
        &mut app.vg,
        crate::draw::snap_box(grown, density),
        crate::ui::corner_radius_all(0.0),
    );
    // Already a 0-to-1 NanoVG colour, like the theme's: passed straight
    // through, never through `nvg_color` (which takes Clay's 0-to-255).
    app.vg.stroke_color(crate::welcome::CTA_RING_INK);
    app.vg.stroke_width(crate::welcome::CTA_RING_WIDTH);
    app.vg.stroke();
}

/// Paints the viewer page's frame: the layout with its tips partition, the
/// canvas and overlay under the Clay content, the menu above it, and every
/// post-pass paint. The pass keeps the port's premultiplied blend (opaque
/// fills pre-blended on the CPU, as h4-readout settled).
fn draw_viewer(app: &mut App, theme: &mv_theme::Theme, now: crate::app_loop::Time, density: f32) {
    let commands = layout_page(app);
    // The scrub tips float above the rows (the DOM's `z-index: 1` on `.tip`):
    // the commands a laid-out tip's box holds walk after the rows' post-pass
    // rings, which would otherwise print over them (h5-fix-tip: k's inset
    // ring over a's tip). The walks keep Clay's order within each layer; a
    // neighbour's text fully inside the tip keeps its place ahead of the
    // tip's plate in both, so it paints the same.
    // The scrub tips' four containers plus the P edit tip's (h8-points-field):
    // one layer above the rows' rings, as the DOM stacks positioned tips.
    let mut tip_ids = crate::scrub_value::tip_box_ids().to_vec();
    tip_ids.push(crate::x_field::tip_id());
    let mut tip_rects: Vec<clay::BoundingBox> = Vec::new();
    for command in &commands {
        if command.commandType == clay::RenderCommandType::Rectangle
            && tip_ids.contains(&command.id)
        {
            tip_rects.push(command.boundingBox);
        }
    }
    let (tips, rest): (Vec<clay::RenderCommand>, Vec<clay::RenderCommand>) =
        commands.into_iter().partition(|command| {
            tip_rects
                .iter()
                .any(|tip| contains_box(*tip, &command.boundingBox))
        });
    // The canvas first: the grid, curves and overlay paint under the
    // page's Clay content, as the canvas paints under the DOM that floats
    // over it. The floating menu (and its shadow) then paint over the
    // canvas they overlap, as the popover stacks above the page. The bar
    // and sidebar shadows keep their place under the canvas, as before.
    // The canvas's region first, then the overlay button's, so the button
    // wins their overlap at equal z.
    crate::plane_input::declare_plane_region(app);
    declare_plane_button(app);
    crate::input::collect_regions(app);
    paint_bar_shadow(app, theme);
    paint_sidebar_shadow(app, theme);
    draw_plane(app, theme, now);
    paint_menu_shadow(app, theme);
    draw_commands(app, density, &rest);
    // The rows' swatches, then the gear's hover border and focus ring.
    crate::settings::paint_swatches(app);
    crate::settings::paint_chrome(app);
    // The readout's two lines, scaled to fit where the equation is long.
    crate::readout::paint_readout(app);
    // The transform rows' inset rings, focus rings and chevrons (h4-rows),
    // and Reset all's focus ring (h5-buttons).
    crate::transform_row::paint_rows(app);
    // The x-of-P slider's track, thumb and tip (h8-points-slider; the Points
    // card's paint hook, beside the rows).
    crate::point_slider::paint_slider(app);
    crate::transform_section::paint_head(app);
    // The Points card's rows (h8-points-row): the marks, letters, badge,
    // roles and readouts, over the rows beneath like the ruler's tape.
    crate::points_section::paint_points(app);
    // The scrub ruler's tape under a dragged box (h5-ruler): over the rows
    // beneath, as the positioned tape stacks in the DOM.
    crate::ruler::paint_open_drag(app);
    // The tip layer above the rings (partitioned above): the tip's plate,
    // caps and words cover whatever ring segments run beneath them, as the
    // DOM paints the positioned tip over the boxes. The arrows land on the
    // caps last, as `paint_rows` held them.
    draw_commands(app, density, &tips);
    crate::scrub_value::paint_extras(app);
    // The P edit tip's arrows on its plate, as the scrub tip's
    // (h8-points-field).
    crate::x_field::paint_tip_arrows(app);
    // The canvas's own cursor wins per frame (h7-plane): the regions'
    // cursor doesn't know the hover rule (a diamond-to-square jump leaves
    // pan under the pointer), so the state rule re-resolves after every
    // layout's retarget.
    crate::plane_input::plane_cursor(app);
}

/// Paints the top bar's shadow: 0 6 px 18 px in the shadow colour
/// (`--shadow-card`), skipped where transparent (the dark themes).
fn paint_bar_shadow(app: &mut App, theme: &mv_theme::Theme) {
    // Theme colours run 0 to 1, as NanoVG takes them; only Clay's 0-to-255
    // colours go through `nvg_color`.
    let ink = theme.colors[mv_theme::Token::ShadowColor];
    if ink[3] <= 0.0 {
        return;
    }
    let Some(b) = element_box(app, mv_clay::id("TopBar", 0)) else {
        return;
    };
    paint_shadow(app, b, crate::top_bar::TOPBAR_RADIUS, ink);
}

/// Paints the sidebar's shadow: 0 6 px 18 px in the shadow colour
/// (`--shadow-card`), skipped where transparent (the dark themes).
fn paint_sidebar_shadow(app: &mut App, theme: &mv_theme::Theme) {
    let ink = theme.colors[mv_theme::Token::ShadowColor];
    if ink[3] <= 0.0 {
        return;
    }
    let Some(b) = element_box(app, mv_clay::id(SIDEBAR_ID, 0)) else {
        return;
    };
    paint_shadow(app, b, SIDEBAR_RADIUS, ink);
}

/// Paints the settings menu's shadow: 0 6 px 18 px in the shadow colour,
/// skipped where transparent (the dark themes) and while closed.
fn paint_menu_shadow(app: &mut App, theme: &mv_theme::Theme) {
    if !app.settings.open {
        return;
    }
    let ink = theme.colors[mv_theme::Token::ShadowColor];
    if ink[3] <= 0.0 {
        return;
    }
    let Some(b) = element_box(app, mv_clay::id(crate::settings::layout::MENU_ID, 0)) else {
        return;
    };
    paint_shadow(app, b, crate::settings::layout::MENU_RADIUS, ink);
}

/// The laid-out box of an element, if the frame laid it out.
fn element_box(app: &App, id: clay::ElementId) -> Option<clay::BoundingBox> {
    let data = app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(id);
    data.found.then_some(data.boundingBox)
}

/// Whether the tip's box holds the command's box: the tip's own plate and
/// border share its box exactly, its caps and words sit strictly inside.
/// Neighbour boxes only overlap it (their far edges stick out), so exact
/// comparison needs no slack.
fn contains_box(tip: clay::BoundingBox, inner: &clay::BoundingBox) -> bool {
    tip.x <= inner.x
        && tip.y <= inner.y
        && inner.x + inner.width <= tip.x + tip.width
        && inner.y + inner.height <= tip.y + tip.height
}

/// Paints a 0 6 px 18 px shadow behind the box.
fn paint_shadow(app: &mut App, b: clay::BoundingBox, radius: f32, ink: mv_nvg::Color) {
    let shadow = ink;
    let transparent = [shadow[0], shadow[1], shadow[2], 0.0];
    let blur = 18.0;
    let feather = ((blur / 2.0) * (2.0 * std::f32::consts::PI).sqrt()).max(1.0);
    let paint = app.vg.box_gradient(
        b.x,
        b.y + 6.0,
        b.width,
        b.height,
        radius,
        feather,
        shadow,
        transparent,
    );
    app.vg.begin_path();
    app.vg.rect(
        b.x - blur * 2.0,
        b.y + 6.0 - blur * 2.0,
        b.width + blur * 4.0,
        b.height + blur * 4.0,
    );
    crate::draw::rounded_rect(&mut app.vg, b, crate::ui::corner_radius_all(radius));
    app.vg.path_winding(mv_nvg::Winding::CW);
    app.vg.fill_paint(&paint);
    app.vg.fill();
}

/// Paints Clay's render commands: boxes, borders, text and icons.
fn draw_commands(app: &mut App, density: f32, commands: &[clay::RenderCommand]) {
    for command in commands {
        let mut bb = command.boundingBox;
        match command.commandType {
            clay::RenderCommandType::Rectangle | clay::RenderCommandType::Border => {
                bb = crate::draw::snap_box(bb, density);
            }
            _ => {}
        }
        match command.commandType {
            clay::RenderCommandType::Rectangle => {
                let data = mv_clay::rect_of(command).expect("Rectangle command holds a rectangle");
                app.vg.begin_path();
                crate::draw::rounded_rect(&mut app.vg, bb, data.cornerRadius);
                app.vg
                    .fill_color(crate::draw::css_background(crate::draw::nvg_color(
                        data.backgroundColor,
                    )));
                app.vg.fill();
            }
            clay::RenderCommandType::Border => {
                let data = mv_clay::border_of(command).expect("Border command holds a border");
                let width = data.width.top as f32;
                let inset = clay::BoundingBox {
                    x: bb.x + width / 2.0,
                    y: bb.y + width / 2.0,
                    width: (bb.width - width).max(0.0),
                    height: (bb.height - width).max(0.0),
                };
                app.vg.begin_path();
                crate::draw::rounded_rect(
                    &mut app.vg,
                    inset,
                    crate::draw::inset_radius(data.cornerRadius, width / 2.0),
                );
                app.vg.stroke_color(crate::draw::nvg_color(data.color));
                app.vg.stroke_width(width);
                app.vg.stroke();
            }
            clay::RenderCommandType::Text => {
                crate::typography::draw_text(app, command);
            }
            clay::RenderCommandType::ScissorStart => {
                app.vg.scissor(bb.x, bb.y, bb.width, bb.height);
            }
            clay::RenderCommandType::ScissorEnd => {
                app.vg.reset_scissor();
            }
            clay::RenderCommandType::Custom => {
                if let Some((icon, _)) = crate::icons::custom_of(command) {
                    crate::icons::draw_icon(
                        &mut app.vg,
                        [bb.x, bb.y],
                        bb.width.min(bb.height),
                        icon,
                    );
                }
            }
            clay::RenderCommandType::None
            | clay::RenderCommandType::Image
            | clay::RenderCommandType::OverlayColorStart
            | clay::RenderCommandType::OverlayColorEnd => {}
        }
    }
    app.vg.reset_scissor();
}

/// Stores the just-drawn frame's pixels, top-down, for a future fade.
fn snapshot_frame(app: &mut App) {
    let [w, h] = app.pixels;
    if w <= 0 || h <= 0 {
        return;
    }
    let mut rows = app.vg.read_pixels(w, h);
    // `glReadPixels` reads bottom-up; the fade's image wants top-down: row
    // `y` trades with row `h - 1 - y`.
    let stride = w as usize * 4;
    for y in 0..h as usize / 2 {
        let (top, bottom) = rows.split_at_mut((h as usize - 1 - y) * stride);
        top[y * stride..(y + 1) * stride].swap_with_slice(&mut bottom[..stride]);
    }
    app.transition.snapshot = rows;
    app.transition.snapshot_size = [w, h];
}

/// Paints the fade: the snapshot over the new frame at falling alpha.
/// The first fading frame uploads the image; the last deletes it.
fn paint_transition(
    app: &mut App,
    active: crate::transition::ActiveTransition,
    progress: f32,
    now: crate::app_loop::Time,
) {
    let [w, h] = app.pixels;
    let snapshot_ok = !app.transition.snapshot.is_empty() && app.transition.snapshot_size == [w, h];
    if !snapshot_ok {
        app.transition.active = None;
        return;
    }
    if app.transition.image == 0 {
        let image = app
            .vg
            .create_image_rgba(w, h, 0, Some(&app.transition.snapshot));
        app.transition.image = image;
    }
    let image = app.transition.image;
    if image != 0 {
        // The snapshot is device pixels; the paint maps it onto the page's
        // points, as the layer composite does.
        let paint = app.vg.image_pattern(
            0.0,
            0.0,
            app.size[0],
            app.size[1],
            0.0,
            image,
            1.0 - progress,
        );
        app.vg.begin_path();
        app.vg.rect(0.0, 0.0, app.size[0], app.size[1]);
        // The snapshot is in device pixels; the frame is laid out in points.
        app.vg.fill_paint(&paint);
        app.vg.fill();
    }
    if Transition::is_done(active, now) {
        if app.transition.image != 0 {
            app.vg.delete_image(app.transition.image);
            app.transition.image = 0;
        }
        app.transition.active = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// During a reset the shift diamond sits at the live h, k while the
    /// square still rides the shown transform (the h8-plane fix-forward from
    /// h7-sidebar's pixels review: Reset all with h = 2 draws the diamond at
    /// h = 0, as the reference does).
    #[test]
    fn reset_diamond_sits_at_the_live_vertex() {
        let mut app = crate::fixture::headless_app();
        app.h_param.set_flushed(&mut app.url, 0.0);
        app.state.jumps.set(vec![crate::state::Jump {
            param: crate::state::TransformParam::H,
            from: 2.0,
            to: 0.0,
        }]);
        crate::flip::adopt_jumps(&mut app);
        let live = crate::flip::live_transform(&app);
        let shown = crate::flip::shown_transform(&app.flip, live);
        assert_eq!((live.h, live.k), (0.0, 0.0));
        assert_eq!(shown.h, 2.0);
        let handles = build_handles(&app);
        assert_eq!((handles[0].at.x, handles[0].at.y), (0.0, 0.0));
    }
}
