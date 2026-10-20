//! Page layout ported from `ui.odin` (`init_clay`, `release_clay`,
//! `clay_error`, `current_theme`, `ink`, `PAGE_PADDING`, `PAGE_GAP`,
//! `STACKED_AT`, `STACKED_PLANE_MIN`, `layout`, `after_layout`, `card`).
//!
//! Layout declarations are pure data. Opening elements, closing the layout
//! and reading it back run the `t0c` Clay bindings. [`ui_scope`] is the
//! scope the Odin `UI()(){}` expresses with `defer`: it aliases `App` for
//! the children, which the safe Clay wrapper's signatures would otherwise
//! forbid, exactly as the Odin original does.

use mv_sys::clay;
use mv_theme::{Color, Theme, Token};

use crate::app::App;
use crate::input::region_of;
use crate::scroll::{
    SCROLLER_IDS, Scroller, scroll_container, scroll_into_view, scrolled, update_scroll,
};

/// The page's padding in pixels. Ported from `PAGE_PADDING`.
pub const PAGE_PADDING: f32 = 16.0;
/// The page's child gap in pixels. Ported from `PAGE_GAP`.
pub const PAGE_GAP: f32 = 12.0;
/// The widest window that stacks. Ported from `STACKED_AT`.
pub const STACKED_AT: f32 = 899.0;
/// The stacked plane's minimum height in pixels. Ported from
/// `STACKED_PLANE_MIN`.
pub const STACKED_PLANE_MIN: f32 = 480.0;

/// Sets up Clay with its arena and the text measure. Ported from `init_clay`;
/// needs the `t0c` Clay bindings.
///
/// Must run after the `App`'s last move: the measure closure keeps a
/// `*mut App` (`MeasureApp`) that Clay calls back into for the context's
/// whole life, so moving the `App` afterwards would leave it pointing at the
/// old home. Callers keep the `App` in its final place (a local, or the
/// boxed state) before calling this, and never move it again until
/// `release_clay`.
///
/// The arena size follows `mv_clay::Clay::new`'s contract: it folds
/// `MinMemorySize` in, so passing 0 asks for exactly the minimum, as the Odin
/// code passes `MinMemorySize()` (a smaller arena would fail `Initialize`;
/// app code never calls the C library directly).
pub fn init_clay(app: &mut App) {
    let size = crate::WINDOW_SIZE;
    app.clay_memory = Vec::new();
    let apps = MeasureApp(app as *mut App);
    let clay = mv_clay::Clay::new(
        0,
        clay::Dimensions {
            width: size[0] as f32,
            height: size[1] as f32,
        },
        |error| clay_error(error),
    );
    clay.set_measure_text(move |slice, config| measure_with(apps, slice, config));
    app.clay = Some(clay);
}

/// The app for Clay's measure callback: what Odin passes as the callback's
/// user data on every measure call.
#[derive(Clone, Copy)]
struct MeasureApp(*mut App);

// SAFETY: frames lay out on one thread while the app lives; Clay never moves
// the closure to another thread in this app.
unsafe impl Send for MeasureApp {}
// SAFETY: as above; measures never run concurrently.
unsafe impl Sync for MeasureApp {}

/// Clay's measure through the app: the closure above owns the pointer whole,
/// so the wrapper's thread-safety impls apply.
fn measure_with(
    apps: MeasureApp,
    slice: &clay::StringSlice,
    config: &clay::TextElementConfig,
) -> clay::Dimensions {
    // SAFETY: the app outlives its Clay context, and layouts run on this
    // thread, so no other thread touches the app through this pointer.
    let app = unsafe { &mut *apps.0 };
    crate::typography::measure_text(app, slice, config)
}

/// Lets Clay's memory go. Ported from `release_clay`.
pub fn release_clay(app: &mut App) {
    drop(app.clay.take());
    app.clay_memory = Vec::new();
}

/// Logs a Clay error. Ported from `clay_error`.
pub extern "C" fn clay_error(error: clay::ErrorData) {
    eprintln!("clay {:?}: {}", error.errorType, error.message());
}

/// The theme shown: the pick for the scheme in use, from the themes resolved
/// at start. Ported from `current_theme`.
pub fn current_theme(app: &App) -> &Theme {
    &app.themes[mv_theme::shown(app.prefs, app.system_scheme)]
}

/// A theme colour as Clay takes it, 0 to 255. Ported from `ink`.
pub fn ink(t: &Theme, token: Token) -> Color {
    let c = t.colors[token];
    [c[0] * 255.0, c[1] * 255.0, c[2] * 255.0, c[3] * 255.0]
}

/// Lays out the page: the top bar across, then the panel beside the plane, or
/// stacked above it in a narrow window with the page scrolling. Ported from
/// `layout`; needs the `t0c` Clay bindings.
pub fn layout(app: &mut App, size: [f32; 2]) -> Vec<clay::RenderCommand> {
    let theme = current_theme(app).clone();
    let stacked = size[0] <= STACKED_AT;
    scroll_container(app, Scroller::Page, stacked, 0.0);
    scroll_container(app, Scroller::Panel, !stacked, 1.0);
    {
        let clay = app.clay.as_mut().expect("clay context initialised");
        clay.set_layout_dimensions(clay::Dimensions {
            width: size[0],
            height: size[1],
        });
        let pressed = app.input.pressed.kind != crate::input::HitKind::None;
        clay.set_pointer_state(app.input.pointer, pressed);
    }
    app.input.inert_layers = 0;
    {
        let clay = app.clay.as_mut().expect("clay context initialised");
        clay.begin_layout();
    }
    let page_id = mv_clay::id(SCROLLER_IDS[Scroller::Page], 0);
    let mut page = empty_decl();
    page.layout = clay::LayoutConfig {
        sizing: clay::Sizing {
            width: sizing_grow(),
            height: sizing_grow(),
        },
        padding: clay::Padding {
            left: PAGE_PADDING as u16,
            right: PAGE_PADDING as u16,
            top: PAGE_PADDING as u16,
            bottom: PAGE_PADDING as u16,
        },
        childGap: PAGE_GAP as u16,
        childAlignment: clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Top,
        },
        layoutDirection: clay::LayoutDirection::TopToBottom,
    };
    page.backgroundColor = ink(&theme, Token::Background);
    let offset = scrolled(app, Scroller::Page);
    page.clip = clay::ClipElementConfig {
        horizontal: false,
        vertical: stacked,
        childOffset: offset,
    };
    ui_scope(app, page_id, &page, |app| {
        crate::top_bar::top_bar(app, &theme, size[0], size[0] - 2.0 * PAGE_PADDING);
        if stacked {
            crate::sidebar::panel(app, &theme, stacked);
            crate::plane_area::plane_area(app, &theme, STACKED_PLANE_MIN);
        } else {
            let body_id = mv_clay::id("Body", 0);
            let mut body = empty_decl();
            body.layout = clay::LayoutConfig {
                sizing: clay::Sizing {
                    width: sizing_grow(),
                    height: sizing_grow(),
                },
                padding: clay::Padding {
                    left: 0,
                    right: 0,
                    top: 0,
                    bottom: 0,
                },
                childGap: PAGE_GAP as u16,
                childAlignment: clay::ChildAlignment {
                    x: clay::LayoutAlignmentX::Left,
                    y: clay::LayoutAlignmentY::Top,
                },
                layoutDirection: clay::LayoutDirection::LeftToRight,
            };
            ui_scope(app, body_id, &body, |app| {
                crate::sidebar::panel(app, &theme, stacked);
                crate::plane_area::plane_area(app, &theme, 0.0);
            });
        }
    });
    // Over the page: the explainer, placed beside its chip.
    crate::explainer::explainer(app, &theme);
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

/// What the layout reports back once Clay has placed everything. True when
/// any of it changed, so the frame lays out again. Ported from
/// `after_layout`; needs `t0c`'s Clay plus the sibling plane, equation-width
/// and explainer readers.
pub fn after_layout(app: &mut App) -> bool {
    let mut changed = update_scroll(app);
    // A part the keyboard focused comes into view, from this layout, which
    // shows what the focus changed.
    if app.input.reveal {
        app.input.reveal = false;
        let focus = app.input.focus;
        if let (r, true) = region_of(app, focus) {
            changed |= scroll_into_view(app, r);
        }
    }
    if app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(mv_clay::id("Plane", 0))
        .found
    {
        let data = app
            .clay
            .as_ref()
            .expect("clay context initialised")
            .get_element_data(mv_clay::id("Plane", 0));
        changed |= crate::plane_area::plane_after_layout(app, data.boundingBox);
    }
    {
        let data = app
            .clay
            .as_ref()
            .expect("clay context initialised")
            .get_element_data(mv_clay::id("Equation", 0));
        if data.found && data.boundingBox.width != app.equation_width {
            app.equation_width = data.boundingBox.width;
            changed = true;
        }
    }
    // The explainer is placed from a layout that has settled.
    if !changed {
        changed = crate::explainer::explainer_after_layout(app);
    }
    changed
}

/// A card of the page: `--background-muted` inside a 1 px `--border` line.
/// Ported from `card`.
pub fn card(t: &Theme) -> (clay::Color, clay::BorderElementConfig, clay::CornerRadius) {
    (
        ink(t, Token::BackgroundMuted),
        clay::BorderElementConfig {
            color: ink(t, Token::Border),
            width: clay::BorderWidth {
                left: 1,
                right: 1,
                top: 1,
                bottom: 1,
                betweenChildren: 0,
            },
        },
        corner_radius_all(t.card_radius),
    )
}

/// The scope the Odin `UI()(){}` expresses with `defer`: opens the element,
/// runs the children, closes it. Ported from Clay's `UI` scope in `layout`.
pub(crate) fn ui_scope(
    app: &mut App,
    id: clay::ElementId,
    decl: &clay::ElementDeclaration,
    children: impl FnOnce(&mut App),
) {
    let clay = app.clay.as_ref().expect("clay context initialised") as *const mv_clay::Clay;
    let app_ptr = app as *mut App;
    // SAFETY: the frame lays out on this thread while the app lives; Clay
    // invokes the children synchronously before `ui` returns and retains
    // neither pointer. Scopes nest (the page's scope around the body's in
    // `layout`), so borrows from one raw pointer do overlap while nested:
    // the real invariant is strict LIFO nesting on one thread. An inner
    // scope's reborrowed `&mut App` ends before the outer children's run
    // touches its own again, so the two are never used at once; the measure
    // callback reaches the app through its own pointer and runs at
    // `end_layout`, outside every children's run, never beside one. This is
    // the aliasing the Odin `UI()(){children}` scope has by design; the
    // wrapper keeps it at this one call site instead of every layout body.
    unsafe {
        (*clay).ui(id, decl, || children(&mut *app_ptr));
    }
}

/// A blank element declaration: every field the Odin `{...}` literal leaves
/// unset, spelled out, since Rust has no zero-initialised struct literals.
/// Clay's zero sizing is FIT (`SizingType::Fit = 0`), so the blank defaults
/// to FIT on both axes; callers that grow say so explicitly. The zero
/// min/max pair lives with the top bar (`top_bar::ZERO_MINMAX`): one
/// constant for the whole crate.
pub(crate) fn empty_decl() -> clay::ElementDeclaration {
    clay::ElementDeclaration {
        layout: clay::LayoutConfig {
            sizing: clay::Sizing {
                width: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
                height: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
            },
            padding: clay::Padding {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
            },
            childGap: 0,
            childAlignment: clay::ChildAlignment {
                x: clay::LayoutAlignmentX::Left,
                y: clay::LayoutAlignmentY::Top,
            },
            layoutDirection: clay::LayoutDirection::LeftToRight,
        },
        backgroundColor: [0.0; 4],
        overlayColor: [0.0; 4],
        cornerRadius: clay::CornerRadius {
            topLeft: 0.0,
            topRight: 0.0,
            bottomLeft: 0.0,
            bottomRight: 0.0,
        },
        aspectRatio: clay::AspectRatioElementConfig { aspectRatio: 0.0 },
        image: clay::ImageElementConfig {
            imageData: std::ptr::null_mut(),
        },
        floating: clay::FloatingElementConfig {
            offset: [0.0, 0.0],
            expand: clay::Dimensions {
                width: 0.0,
                height: 0.0,
            },
            parentId: 0,
            zIndex: 0,
            attachment: clay::FloatingAttachPoints {
                element: clay::FloatingAttachPointType::LeftTop,
                parent: clay::FloatingAttachPointType::LeftTop,
            },
            pointerCaptureMode: clay::PointerCaptureMode::Capture,
            attachTo: clay::FloatingAttachToElement::None,
            clipTo: clay::FloatingClipToElement::None,
        },
        custom: clay::CustomElementConfig {
            customData: std::ptr::null_mut(),
        },
        clip: clay::ClipElementConfig {
            horizontal: false,
            vertical: false,
            childOffset: [0.0, 0.0],
        },
        border: clay::BorderElementConfig {
            color: [0.0; 4],
            width: clay::BorderWidth {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
                betweenChildren: 0,
            },
        },
        transition: clay::TransitionElementConfig {
            handler: None,
            duration: 0.0,
            properties: 0,
            interactionHandling:
                clay::TransitionInteractionHandlingType::DisableInteractionsWhileTransitioningPosition,
            enter: clay::TransitionEnterConfig {
                setInitialState: None,
                trigger: clay::TransitionEnterTriggerType::SkipOnFirstParentFrame,
            },
            exit: clay::TransitionExitConfig {
                setFinalState: None,
                trigger: clay::TransitionExitTriggerType::SkipWhenParentExits,
                siblingOrdering: clay::ExitTransitionSiblingOrdering::UnderneathSiblings,
            },
        },
        userData: std::ptr::null_mut(),
    }
}

/// A grow sizing axis. Mirrors Clay's `SizingGrow` with a real value, so
/// layout bodies don't depend on `t0c`'s helper bodies for pure data.
pub(crate) fn sizing_grow() -> clay::SizingAxis {
    clay::SizingAxis {
        constraints: clay::SizingConstraints {
            sizeMinMax: clay::SizingConstraintsMinMax { min: 0.0, max: 0.0 },
        },
        r#type: clay::SizingType::Grow,
    }
}

/// A fixed sizing axis of `v` pixels. Mirrors Clay's `SizingFixed`.
pub(crate) fn sizing_fixed(v: f32) -> clay::SizingAxis {
    clay::SizingAxis {
        constraints: clay::SizingConstraints {
            sizeMinMax: clay::SizingConstraintsMinMax { min: v, max: v },
        },
        r#type: clay::SizingType::Fixed,
    }
}

/// A corner radius on all four corners. Mirrors Clay's `CornerRadiusAll`.
pub(crate) fn corner_radius_all(v: f32) -> clay::CornerRadius {
    clay::CornerRadius {
        topLeft: v,
        topRight: v,
        bottomLeft: v,
        bottomRight: v,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use enum_map::Enum;

    /// Boxes hold within 0.02 px: the engines lay out in 1/64 px.
    /// Ported from `layout_test.odin:32` (`SLACK`).
    const SLACK: f32 = 0.02;
    /// Boxes sized by text hold within 0.1 px: the port's advances are within
    /// 0.06 px of Chromium's, rounded up to 1/64 px. Ported from
    /// `layout_test.odin:38` (`TEXT_SLACK`).
    const TEXT_SLACK: f32 = 0.1;
    /// Baselines hold within one layout unit: WebKit floors a baseline to it.
    /// Ported from `layout_test.odin:41` (`BASELINE_SLACK`).
    const BASELINE_SLACK: f32 = 1.0 / 64.0;

    /// One named part of the page: the fixture's box it must sit in.
    /// Ported from `layout_test.odin:44` (`Part`).
    struct Part {
        name: String,
        id: clay::ElementId,
        slack: f32,
        /// Held to Chromium's box alone, where the port takes Chromium's
        /// pick (the gear's glyph, call 3 l).
        chromium: bool,
    }

    /// Every part of the page the fixture measures. Ported from
    /// `layout_test.odin:52` (`parts`).
    fn parts() -> Vec<Part> {
        let mut list = Vec::new();
        let add = |list: &mut Vec<Part>,
                   name: String,
                   id: clay::ElementId,
                   slack: f32,
                   chromium: bool| {
            list.push(Part {
                name,
                id,
                slack,
                chromium,
            });
        };
        add(
            &mut list,
            "topbar".into(),
            mv_clay::id("TopBar", 0),
            SLACK,
            false,
        );
        add(
            &mut list,
            "wordmark".into(),
            mv_clay::id("Wordmark", 0),
            TEXT_SLACK,
            false,
        );
        add(
            &mut list,
            "wordmark_icon".into(),
            mv_clay::id("WordmarkIcon", 0),
            SLACK,
            false,
        );
        add(
            &mut list,
            "separator".into(),
            mv_clay::id("Separator", 0),
            TEXT_SLACK,
            false,
        );
        add(
            &mut list,
            "title_block".into(),
            mv_clay::id("TitleBlock", 0),
            TEXT_SLACK,
            false,
        );
        add(
            &mut list,
            "chip".into(),
            mv_clay::id("ThemeChip", 0),
            TEXT_SLACK,
            false,
        );
        add(
            &mut list,
            "chip_dot".into(),
            mv_clay::id("ChipDot", 0),
            TEXT_SLACK,
            false,
        );
        add(
            &mut list,
            "gear".into(),
            mv_clay::id("Gear", 0),
            TEXT_SLACK,
            false,
        );
        add(
            &mut list,
            "gear_icon".into(),
            mv_clay::id("GearIcon", 0),
            SLACK,
            true,
        );
        add(
            &mut list,
            "panel".into(),
            mv_clay::id("Panel", 0),
            SLACK,
            false,
        );
        add(
            &mut list,
            "plane".into(),
            mv_clay::id("Plane", 0),
            SLACK,
            false,
        );
        for i in 0..mv_viewer::Section::LENGTH {
            let s = mv_viewer::Section::from_usize(i);
            add(
                &mut list,
                format!("card_{}", mv_viewer::SECTION_TITLES[s].to_lowercase()),
                mv_clay::id("Card", i as u32),
                SLACK,
                false,
            );
        }
        add(
            &mut list,
            "picker".into(),
            mv_clay::id("Picker", 0),
            SLACK,
            false,
        );
        for i in 0..mv_viewer::Base::LENGTH {
            add(
                &mut list,
                format!("option_{i}"),
                mv_clay::id("PickerOption", i as u32),
                SLACK,
                false,
            );
        }
        add(
            &mut list,
            "equation".into(),
            mv_clay::id("Equation", 0),
            SLACK,
            false,
        );
        for i in 0..mv_viewer::Param::LENGTH {
            let p = mv_viewer::Param::from_usize(i);
            let l = mv_viewer::PARAM_LETTERS[p];
            let u = i as u32;
            add(
                &mut list,
                format!("row_{l}"),
                mv_clay::id("Row", u),
                SLACK,
                false,
            );
            add(
                &mut list,
                format!("chip_{l}"),
                mv_clay::id("Chip", u),
                SLACK,
                false,
            );
            add(
                &mut list,
                format!("name_slot_{l}"),
                mv_clay::id("NameSlot", u),
                SLACK,
                false,
            );
            if mv_viewer::is_scale(p) {
                add(
                    &mut list,
                    format!("flip_{l}"),
                    mv_clay::id("Flip", u),
                    SLACK,
                    false,
                );
            }
            add(
                &mut list,
                format!("box_{l}"),
                mv_clay::id("Box", u),
                SLACK,
                false,
            );
        }
        for i in 0..mv_viewer::Point::LENGTH {
            let point = mv_viewer::Point::from_usize(i);
            let l = mv_viewer::POINT_NAMES[point].to_lowercase();
            let u = i as u32;
            add(
                &mut list,
                format!("point_{l}"),
                mv_clay::id("Point", u),
                SLACK,
                false,
            );
            add(
                &mut list,
                format!("head_{l}"),
                mv_clay::id("PointHead", u),
                SLACK,
                false,
            );
            add(
                &mut list,
                format!("mark_{l}"),
                mv_clay::id("Mark", u),
                SLACK,
                false,
            );
            add(
                &mut list,
                format!("role_slot_{l}"),
                mv_clay::id("RoleSlot", u),
                SLACK,
                false,
            );
            add(
                &mut list,
                format!("readout_{l}"),
                mv_clay::id("Readout", u),
                SLACK,
                false,
            );
        }
        add(
            &mut list,
            "track".into(),
            mv_clay::id("Track", 0),
            SLACK,
            false,
        );
        list
    }

    /// Whether the port's box sits in the reference's within `slack`.
    /// Ported from `layout_test.odin:111` (`near_box`).
    fn near_box(port: clay::BoundingBox, want: [f32; 4], slack: f32) -> bool {
        (port.x - want[0]).abs() <= slack
            && (port.y - want[1]).abs() <= slack
            && (port.width - want[2]).abs() <= slack
            && (port.height - want[3]).abs() <= slack
    }

    /// The port's text: its pen, the baseline it sits on, and whether it is
    /// drawn at all (a name or role that doesn't fit its slot isn't).
    /// Ported from `layout_test.odin:123` (`Port_Text`).
    struct PortText {
        x: f32,
        baseline: f32,
        shown: bool,
    }

    /// A Clay text element's run in the frame, found by its words. Ported
    /// from `layout_test.odin:130` (`clay_text`).
    fn clay_text(app: &mut App, commands: &[clay::RenderCommand], s: &str) -> Option<PortText> {
        for c in commands {
            // The payload through `text_of`, as the draw reads it: a Text
            // command's payload is text; the slice borrows Clay's laid-out
            // string for the comparison below, as `typography::draw_text`
            // reads it for the draw.
            let Some(data) = mv_clay::text_of(c) else {
                continue;
            };
            // SAFETY: as above.
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    data.stringContents.chars as *const u8,
                    data.stringContents.length as usize,
                )
            };
            if std::str::from_utf8(bytes).unwrap_or("") != s {
                continue;
            }
            let style = crate::typography::clay_type_style(app, data.fontId);
            let b = c.boundingBox;
            return Some(PortText {
                x: b.x,
                baseline: b.y
                    + mv_text::baseline(
                        &mut app.fonts,
                        style.style.face,
                        style.style.size,
                        b.height,
                    ),
                shown: true,
            });
        }
        None
    }

    /// A `Text_Line` element's run: drawn when its text fits its box, or cut
    /// with an ellipsis. Ported from `layout_test.odin:156` (`line_text`).
    fn line_text(
        app: &mut App,
        id: clay::ElementId,
        ty: crate::typography::Type,
        s: &str,
        overflow: crate::typography::Overflow,
    ) -> Option<PortText> {
        let data = mv_clay::element_data(id);
        if !data.found {
            return None;
        }
        let b = data.boundingBox;
        let style = crate::typography::type_style(ty, current_theme(app));
        let shown_text;
        let shown = if style.caps {
            shown_text = s.to_uppercase();
            &shown_text
        } else {
            s
        };
        let fits = mv_text::layout_width(mv_text::measure(&mut app.fonts, style.style, shown))
            <= b.width + 1e-3;
        let baseline = b.y
            + mv_text::baseline(
                &mut app.fonts,
                style.style.face,
                style.style.size,
                style.line_height,
            );
        Some(PortText {
            x: b.x,
            baseline,
            shown: fits || overflow == crate::typography::Overflow::Ellipsis,
        })
    }

    /// The page's layout against the reference's own
    /// (`testdata/page.json`, written by `tools/run dom --fixture`): at
    /// every shot the fixture holds, every part's box is held to the
    /// reference's, and texts by their pen and baseline. Ported from
    /// `layout_test.odin:179` (`layout_is_the_references`), expectations
    /// and slacks byte-identical to the Odin's.
    #[test]
    fn layout_is_the_references() {
        let _guard = mv_clay::tests_lock();
        let page = crate::fixture::page_fixture().expect("page fixture");
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        for i in 0..mv_theme::Slug::LENGTH {
            let slug = mv_theme::Slug::from_usize(i);
            app.themes[slug] = mv_theme::resolve(slug, mv_theme::Space::Srgb);
        }
        app.system_scheme = mv_theme::Scheme::Dark;
        app.prefs = mv_theme::DEFAULT_PREFS;
        app.viewer = mv_viewer::initial_state();
        crate::plane_area::plane_init(&mut app);
        // After the final placement: the measure closure keeps a `*mut App`
        // (see `init_clay`), so the `App` must not move past this point.
        init_clay(&mut app);

        let mut failures: Vec<String> = Vec::new();
        let mut names: Vec<String> = page.shots.keys().cloned().collect();
        names.sort();
        for name in &names {
            let shot = crate::fixture::fixture_shot(&page, name).expect("fixture shot");
            let (wk, cr) = (&shot.webkit, &shot.chromium);
            app.size = shot.size;
            app.pixels = [(shot.size[0] * 2.0) as i32, (shot.size[1] * 2.0) as i32];
            app.viewer.view = None;
            app.equation_width = 0.0;
            for s in [Scroller::Page, Scroller::Panel] {
                app.scroll[s].offset = 0.0;
            }
            if let Some(scrolled) = wk.scroll.get("panel") {
                app.scroll[Scroller::Panel].offset = scrolled.top;
            }
            let commands = crate::draw::settle_layout(&mut app, shot.size);

            let mut checked = 0;
            let mut passed = 0;
            for part in parts() {
                let want_wk = wk.boxes.get(&part.name);
                let data = mv_clay::element_data(part.id);
                let Some(want_wk) = want_wk else {
                    // What the reference hides (the chip under 640 px), the
                    // port declares not.
                    if data.found {
                        failures.push(format!(
                            "{name}: {} shows; the reference hides it",
                            part.name
                        ));
                    }
                    continue;
                };
                checked += 1;
                if !data.found {
                    failures.push(format!("{name}: no {} in the port", part.name));
                    continue;
                }
                let b = data.boundingBox;
                let ok = (!part.chromium && near_box(b, *want_wk, part.slack))
                    || (cr
                        .boxes
                        .get(&part.name)
                        .is_some_and(|want_cr| near_box(b, *want_cr, part.slack)));
                if ok {
                    passed += 1;
                } else {
                    let want_cr = cr.boxes.get(&part.name).copied().unwrap_or([0.0; 4]);
                    failures.push(format!(
                        "{name}: {} at ({:.3}, {:.3}, {:.3} x {:.3}); WebKit's ({:.3}, {:.3}, {:.3} x {:.3}), Chromium's ({:.3}, {:.3}, {:.3} x {:.3})",
                        part.name,
                        b.x, b.y, b.width, b.height,
                        want_wk[0], want_wk[1], want_wk[2], want_wk[3],
                        want_cr[0], want_cr[1], want_cr[2], want_cr[3],
                    ));
                }
            }

            struct TextCheck {
                name: String,
                source: String,
                port: Option<PortText>,
            }
            let mut texts: Vec<TextCheck> = Vec::new();
            let add =
                |texts: &mut Vec<TextCheck>, name: &str, source: &str, port: Option<PortText>| {
                    texts.push(TextCheck {
                        name: name.to_string(),
                        source: source.to_string(),
                        port,
                    });
                };
            add(
                &mut texts,
                "wordmark_text",
                crate::top_bar::WORDMARK,
                clay_text(&mut app, &commands, crate::top_bar::WORDMARK),
            );
            add(
                &mut texts,
                "title_text",
                mv_viewer::VIEW_TITLE,
                line_text(
                    &mut app,
                    mv_clay::id("Title", 0),
                    crate::typography::Type::Title,
                    mv_viewer::VIEW_TITLE,
                    crate::typography::Overflow::Ellipsis,
                ),
            );
            add(
                &mut texts,
                "subtitle_text",
                mv_viewer::VIEW_SUBTITLE,
                line_text(
                    &mut app,
                    mv_clay::id("Subtitle", 0),
                    crate::typography::Type::Subtitle,
                    mv_viewer::VIEW_SUBTITLE,
                    crate::typography::Overflow::Ellipsis,
                ),
            );
            let label = mv_theme::SLUG_LABELS[mv_theme::shown(app.prefs, app.system_scheme)];
            add(
                &mut texts,
                "chip_text",
                label,
                clay_text(&mut app, &commands, label),
            );
            for i in 0..mv_viewer::Section::LENGTH {
                let s = mv_viewer::Section::from_usize(i);
                let title = mv_viewer::SECTION_TITLES[s];
                add(
                    &mut texts,
                    &format!("card_title_{}", title.to_lowercase()),
                    title,
                    clay_text(&mut app, &commands, title),
                );
            }
            for i in 0..mv_viewer::Param::LENGTH {
                let p = mv_viewer::Param::from_usize(i);
                let l = mv_viewer::PARAM_LETTERS[p];
                add(
                    &mut texts,
                    &format!("chip_letter_{l}"),
                    l,
                    clay_text(&mut app, &commands, l),
                );
                add(
                    &mut texts,
                    &format!("name_{l}"),
                    mv_viewer::PARAM_NAMES[p],
                    line_text(
                        &mut app,
                        mv_clay::id("Name", i as u32),
                        crate::typography::Type::RowName,
                        mv_viewer::PARAM_NAMES[p],
                        crate::typography::Overflow::Whole,
                    ),
                );
            }
            for i in 0..mv_viewer::Point::LENGTH {
                let point = mv_viewer::Point::from_usize(i);
                let l = mv_viewer::POINT_NAMES[point].to_lowercase();
                add(
                    &mut texts,
                    &format!("letter_{l}"),
                    mv_viewer::POINT_NAMES[point],
                    clay_text(&mut app, &commands, mv_viewer::POINT_NAMES[point]),
                );
                let role = if point == mv_viewer::Point::P {
                    mv_viewer::P_ROLE
                } else {
                    mv_viewer::Q_ROLE_EMPTY
                };
                add(
                    &mut texts,
                    &format!("role_{l}"),
                    role,
                    line_text(
                        &mut app,
                        mv_clay::id("Role", i as u32),
                        crate::typography::Type::Role,
                        role,
                        crate::typography::Overflow::Whole,
                    ),
                );
            }
            for c in texts {
                let want = wk.texts.get(&c.name);
                let Some(want) = want else {
                    let shown = c.port.is_some_and(|port| port.shown);
                    if shown {
                        failures.push(format!(
                            "{name}: {} shown in the port, not shown in the reference",
                            c.name
                        ));
                    }
                    continue;
                };
                // A text the window doesn't show Clay culls.
                if want.y >= shot.size[1] || want.y + want.height <= 0.0 {
                    continue;
                }
                let shown = c.port.as_ref().is_some_and(|port| port.shown);
                if !shown {
                    failures.push(format!(
                        "{name}: {} not shown in the port, shown in the reference",
                        c.name
                    ));
                    continue;
                }
                if c.source != want.text {
                    failures.push(format!(
                        "{name}: {} reads {:?}; the reference's {:?}",
                        c.name, c.source, want.text
                    ));
                }
                checked += 1;
                let port = c.port.as_ref().expect("shown port has its run");
                let ok = ((port.x - want.x).abs() <= SLACK
                    || cr
                        .texts
                        .get(&c.name)
                        .is_some_and(|want_cr| (port.x - want_cr.x).abs() <= SLACK))
                    && (port.baseline - want.baseline).abs() <= BASELINE_SLACK;
                if ok {
                    passed += 1;
                } else {
                    let want_cr_x = cr
                        .texts
                        .get(&c.name)
                        .map(|want_cr| want_cr.x)
                        .unwrap_or(0.0);
                    failures.push(format!(
                        "{name}: {} at {:.3} on {:.3}; WebKit's at {:.3} on {:.3}, Chromium's at {:.3}",
                        c.name, port.x, port.baseline, want.x, want.baseline, want_cr_x
                    ));
                }
            }
            eprintln!(
                "{name} at {} x {}: {passed} of {checked} boxes and texts at the reference's",
                shot.size[0], shot.size[1]
            );
        }
        crate::ui::release_clay(&mut app);
        mv_text::destroy(&mut app.fonts);
        assert!(
            failures.is_empty(),
            "{} layout difference(s):\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}
