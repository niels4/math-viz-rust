//! Clay setup and declaration helpers: `init_clay`, `release_clay`,
//! `clay_error`, `current_theme`, `ink`.
//!
//! Layout declarations are pure data. Opening elements, closing the layout
//! and reading it back run the Clay bindings. [`ui_scope`] is the scope the
//! Odin `UI()(){}` expresses with `defer`: it aliases `App` for the
//! children, which the safe Clay wrapper's signatures would otherwise
//! forbid. The page milestones build their layouts on these helpers.

use mv_sys::clay;
use mv_theme::{Color, Theme, Token};

use crate::app::App;

/// Sets up Clay with its arena and the text measure.
///
/// Must run after the `App`'s last move: the measure closure keeps a
/// `*mut App` (`MeasureApp`) that Clay calls back into for the context's
/// whole life, so moving the `App` afterwards would leave it pointing at the
/// old home. Callers keep the `App` in its final place (a local, or the
/// boxed state) before calling this, and never move it again until
/// `release_clay`.
///
/// The arena size follows `mv_clay::Clay::new`'s contract: it folds
/// `MinMemorySize` in, so passing 0 asks for exactly the minimum (a smaller
/// arena would fail `Initialize`; app code never calls the C library
/// directly).
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

/// Lets Clay's memory go.
pub fn release_clay(app: &mut App) {
    drop(app.clay.take());
    app.clay_memory = Vec::new();
}

/// Logs a Clay error.
pub extern "C" fn clay_error(error: clay::ErrorData) {
    eprintln!("clay {:?}: {}", error.errorType, error.message());
}

/// The theme shown: the pick for the scheme in use, from the themes resolved
/// at start.
pub fn current_theme(app: &App) -> &Theme {
    &app.themes[mv_theme::shown(app.prefs, app.system_scheme)]
}

/// A theme colour as Clay takes it, 0 to 255.
pub fn ink(t: &Theme, token: Token) -> Color {
    let c = t.colors[token];
    [c[0] * 255.0, c[1] * 255.0, c[2] * 255.0, c[3] * 255.0]
}

/// The scope the Odin `UI()(){}` expresses with `defer`: opens the element,
/// runs the children, closes it.
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
    // neither pointer. Scopes nest, so borrows from one raw pointer do
    // overlap while nested: the real invariant is strict LIFO nesting on one
    // thread. An inner scope's reborrowed `&mut App` ends before the outer
    // children's run touches its own again, so the two are never used at
    // once; the measure callback reaches the app through its own pointer and
    // runs at `end_layout`, outside every children's run, never beside one.
    // This is the aliasing the Odin `UI()(){children}` scope has by design;
    // the wrapper keeps it at this one call site instead of every layout body.
    unsafe {
        (*clay).ui(id, decl, || children(&mut *app_ptr));
    }
}

/// A sizing min/max pair pinned at zero: Clay's zero sizing is FIT
/// (`SizingType::Fit = 0`), so a blank declaration built on this defaults to
/// FIT on both axes; callers that grow say so explicitly.
pub(crate) const ZERO_MINMAX: clay::SizingConstraintsMinMax =
    clay::SizingConstraintsMinMax { min: 0.0, max: 0.0 };

/// A blank element declaration: every field the Odin `{...}` literal leaves
/// unset, spelled out, since Rust has no zero-initialised struct literals.
pub(crate) fn empty_decl() -> clay::ElementDeclaration {
    clay::ElementDeclaration {
        layout: clay::LayoutConfig {
            sizing: clay::Sizing {
                width: mv_clay::sizing_fit(ZERO_MINMAX),
                height: mv_clay::sizing_fit(ZERO_MINMAX),
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
/// layout bodies don't depend on helper bodies for pure data.
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
