//! The page: the top bar over an empty body, as `FunctionViewer` mounts it.
//!
//! Ported from `FunctionViewer/FunctionViewer.tsx` and
//! `FunctionViewer.module.css` at `adb0534`: the page grid (16 px padding,
//! 16 px gaps, title row over content row, 400 px sidebar column), the top
//! bar spanning it, and the sidebar stub the later milestones replace. The
//! frame paint walks Clay's commands through NanoVG; the theme cross-fade
//! paints the last stable frame's snapshot over the new one.

use mv_sys::clay;

use crate::app::App;
use crate::transition::Transition;

/// The page grid: 16 px padding and gaps, 400 px sidebar column.
pub const PAGE_PAD: f32 = 16.0;
pub const PAGE_GAP: f32 = 16.0;
pub const SIDEBAR_WIDTH: f32 = 400.0;

/// The sidebar's stub copy at this hash.
pub const SIDEBAR_TEXT: &str = "Sidebar!";

/// The page's sidebar box, for the settings menu's later use and tests.
pub const SIDEBAR_ID: &str = "Sidebar";

/// Lays out the page in `app`'s Clay context and returns the frame's render
/// commands. The caller opens the layout first and ends it after.
pub fn layout_page(app: &mut App) -> Vec<clay::RenderCommand> {
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
            // `--space-card` inside the 1 px border: Clay's border takes no
            // layout space, the browser's `border-box` does.
            side.layout.padding = clay::Padding {
                left: (crate::top_bar::TOPBAR_PAD + crate::top_bar::TOPBAR_BORDER) as u16,
                right: (crate::top_bar::TOPBAR_PAD + crate::top_bar::TOPBAR_BORDER) as u16,
                top: (crate::top_bar::TOPBAR_PAD + crate::top_bar::TOPBAR_BORDER) as u16,
                bottom: (crate::top_bar::TOPBAR_PAD + crate::top_bar::TOPBAR_BORDER) as u16,
            };
            side.backgroundColor = crate::ui::ink(
                crate::ui::current_theme(app),
                mv_theme::Token::BackgroundMuted,
            );
            side.border = clay::BorderElementConfig {
                color: crate::ui::ink(crate::ui::current_theme(app), mv_theme::Token::Border),
                width: mv_clay::border_outside(1),
            };
            crate::ui::ui_scope(app, mv_clay::id(SIDEBAR_ID, 0), &side, |app| {
                crate::typography::text(
                    app,
                    mv_clay::id("SidebarText", 0),
                    crate::typography::Type::Body,
                    SIDEBAR_TEXT,
                    crate::ui::ink(crate::ui::current_theme(app), mv_theme::Token::Foreground),
                    false,
                );
            });
            let mut body = crate::ui::empty_decl();
            body.layout.sizing = clay::Sizing {
                width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
                height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
            };
            crate::ui::ui_scope(app, mv_clay::id("Body", 0), &body, |_| {});
        });
        // The settings menu floats under the gear, above the page.
        crate::settings::menu(app);
    });
    let commands = app
        .clay
        .as_mut()
        .expect("clay context initialised")
        .end_layout()
        .to_vec();
    // collect_regions needs &mut App and &Clay together; the context moves
    // out and back since no callee on its path reads app.clay.
    let clay = app.clay.take().expect("clay context initialised");
    crate::input::collect_regions(app, &clay);
    app.clay = Some(clay);
    commands
}

/// Paints one frame: the page background, the layout, and the transition's
/// fade when one is in flight.
pub fn draw_page(app: &mut App, now: crate::app_loop::Time) {
    crate::icons::clear_customs();
    crate::settings::clear_swatch_customs();
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
    let commands = layout_page(app);
    paint_bar_shadow(app, &theme);
    let fading = app
        .transition
        .active
        .map(|active| (active, Transition::progress(active, now)));
    draw_commands(app, density, &commands);
    // The gear's hover border and focus ring, over the layout.
    crate::settings::paint_chrome(app);
    if let Some((active, progress)) = fading {
        paint_transition(app, active, progress, now);
    } else {
        snapshot_frame(app);
    }
    app.vg.end_frame();
    crate::input::update_cursor(app);
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
    let data = app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(mv_clay::id("TopBar", 0));
    if !data.found {
        return;
    }
    let b = data.boundingBox;
    let shadow = ink;
    let transparent = [shadow[0], shadow[1], shadow[2], 0.0];
    let blur = 18.0;
    let feather = ((blur / 2.0) * (2.0 * std::f32::consts::PI).sqrt()).max(1.0);
    let paint = app.vg.box_gradient(
        b.x,
        b.y + 6.0,
        b.width,
        b.height,
        crate::top_bar::TOPBAR_RADIUS,
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
    crate::draw::rounded_rect(
        &mut app.vg,
        b,
        crate::ui::corner_radius_all(crate::top_bar::TOPBAR_RADIUS),
    );
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
                } else if let Some((swatch, _)) = crate::settings::swatch_of(command) {
                    crate::settings::draw_swatch(app, [bb.x, bb.y], swatch);
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
