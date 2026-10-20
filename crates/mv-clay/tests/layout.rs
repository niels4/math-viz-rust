//! Layout through the safe wrapper: the spike's row and a measured text run.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use mv_clay::{Clay, id, sizing_fixed, sizing_grow, tests_lock};
use mv_sys::clay as sys;

fn dims(w: f32, h: f32) -> sys::Dimensions {
    sys::Dimensions {
        width: w,
        height: h,
    }
}

fn decl(layout: sys::LayoutConfig, color: [f32; 4]) -> sys::ElementDeclaration {
    // Zeroed defaults (all-zero is a valid value for every field); the test
    // only varies the layout and the background color (transparent boxes
    // emit no render commands).
    let mut e: sys::ElementDeclaration = unsafe { core::mem::zeroed() };
    e.layout = layout;
    e.backgroundColor = color;
    e
}

fn row_layout(child_gap: u16) -> sys::LayoutConfig {
    sys::LayoutConfig {
        sizing: sys::Sizing {
            width: sizing_fixed(600.0),
            height: sizing_fixed(100.0),
        },
        padding: sys::Padding {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        },
        childGap: child_gap,
        childAlignment: sys::ChildAlignment {
            x: sys::LayoutAlignmentX::Left,
            y: sys::LayoutAlignmentY::Top,
        },
        layoutDirection: sys::LayoutDirection::LeftToRight,
    }
}

fn rects_of(commands: &[sys::RenderCommand]) -> Vec<(f32, f32, f32, f32)> {
    let mut out = Vec::new();
    for cmd in commands {
        if cmd.commandType == sys::RenderCommandType::Rectangle {
            let b = cmd.boundingBox;
            out.push((b.x, b.y, b.width, b.height));
        }
    }
    out
}

#[test]
fn layout_row_matches_spike() {
    let _guard = tests_lock();
    let mut clay = Clay::new(1 << 20, dims(600.0, 100.0), |e| {
        panic!("clay error: {:?}", e.errorType)
    });
    clay.begin_layout();
    let root = decl(row_layout(12), [30.0, 30.0, 30.0, 255.0]);
    clay.ui(id("root", 0), &root, || {
        let mut a = decl(row_layout(0), [200.0, 60.0, 60.0, 255.0]);
        a.layout.sizing.width = sizing_fixed(200.0);
        a.layout.sizing.height = sizing_fixed(60.0);
        clay.ui(id("a", 0), &a, || {});
        let mut b = decl(row_layout(0), [60.0, 200.0, 60.0, 255.0]);
        let grow = sizing_grow(sys::SizingConstraintsMinMax { min: 0.0, max: 0.0 });
        b.layout.sizing.width = grow;
        b.layout.sizing.height = grow;
        clay.ui(id("b", 0), &b, || {});
    });
    let rects = rects_of(clay.end_layout());
    assert_eq!(
        rects,
        vec![
            (0.0, 0.0, 600.0, 100.0),
            (0.0, 0.0, 200.0, 60.0),
            (212.0, 0.0, 388.0, 100.0),
        ]
    );
}

// --- Error routing (the t0c-b review's dangling-holder regression) ------------------

// The error closure's record: atomics, because the closure is `'static`.
static ERROR_COUNT: AtomicI32 = AtomicI32::new(0);
static SAW_INTERNAL: AtomicBool = AtomicBool::new(false);

fn error_recorder(e: sys::ErrorData) {
    ERROR_COUNT.fetch_add(1, Ordering::SeqCst);
    if e.errorType == sys::ErrorType::InternalError {
        SAW_INTERNAL.store(true, Ordering::SeqCst);
    }
}

/// An out-of-range `command_at` fires the context's error closure.
///
/// Regression test for the t0c-b review's major finding: `Clay::new` stored
/// `&param as *const dyn ..` (a fat pointer at the dead parameter slot) in
/// the C error handler, so the first Clay error called through a dangling
/// pointer. The holder now points into the heap box that moves into the
/// struct, and this test fires the error path (an out-of-range
/// `Clay__Array_RangeCheck`, `clay.h:3998-4004`) against a live context.
#[test]
fn out_of_range_command_fires_the_error_handler() {
    let _guard = tests_lock();
    ERROR_COUNT.store(0, Ordering::SeqCst);
    SAW_INTERNAL.store(false, Ordering::SeqCst);
    let mut clay = Clay::new(1 << 20, dims(600.0, 100.0), error_recorder);
    clay.begin_layout();
    // Opaque, so the frame holds one command to read in range below.
    let root = decl(row_layout(0), [30.0, 30.0, 30.0, 255.0]);
    clay.ui(id("root", 0), &root, || {});
    let commands = clay.end_layout();
    let array = sys::RenderCommandArray {
        capacity: commands.len() as i32,
        length: commands.len() as i32,
        internalArray: commands.as_ptr() as *mut sys::RenderCommand,
    };
    let first = mv_clay::command_at(&array, 0);
    assert_eq!(first.commandType, commands[0].commandType);
    assert_eq!(first.boundingBox, commands[0].boundingBox);
    assert_eq!(ERROR_COUNT.load(Ordering::SeqCst), 0);
    let _ = mv_clay::command_at(&array, 5);
    assert_eq!(ERROR_COUNT.load(Ordering::SeqCst), 1);
    assert!(SAW_INTERNAL.load(Ordering::SeqCst));
}

// --- Text measurement ---------------------------------------------------------------

static MEASURED: AtomicBool = AtomicBool::new(false);

/// `set_measure_text` runs during layout: the closure outlives the call.
///
/// The second half of the t0c-b review's major finding: `set_measure_text`
/// stored `&measure` (the dead parameter slot) as the C callback's user
/// data. The closure now leaks (no unregister hook on the C side), and this
/// test lays text out through it.
#[test]
fn measure_text_runs_during_layout() {
    let _guard = tests_lock();
    MEASURED.store(false, Ordering::SeqCst);
    let mut clay = Clay::new(1 << 20, dims(600.0, 100.0), error_recorder);
    clay.set_measure_text(|_slice, _config| {
        MEASURED.store(true, Ordering::SeqCst);
        sys::Dimensions {
            width: 42.0,
            height: 10.0,
        }
    });
    clay.begin_layout();
    let root = decl(row_layout(12), [30.0, 30.0, 30.0, 255.0]);
    clay.ui(id("root", 0), &root, || {
        let config: sys::TextElementConfig = unsafe { core::mem::zeroed() };
        clay.text("hi", &config);
    });
    let commands = clay.end_layout();
    assert!(MEASURED.load(Ordering::SeqCst));
    assert!(
        commands
            .iter()
            .any(|c| c.commandType == sys::RenderCommandType::Text)
    );
}

// --- Payload readers ------------------------------------------------------------------

/// One layout with a rectangle, a border, a text and a custom element, each
/// read back through its accessor, plus the free-function element queries.
#[test]
fn payload_readers_read_each_command_back() {
    let _guard = tests_lock();
    MEASURED.store(false, Ordering::SeqCst);
    let mut clay = Clay::new(1 << 20, dims(600.0, 100.0), error_recorder);
    clay.set_measure_text(|_slice, _config| {
        MEASURED.store(true, Ordering::SeqCst);
        sys::Dimensions {
            width: 42.0,
            height: 10.0,
        }
    });
    let payload: u64 = 0xDEAD;
    clay.begin_layout();
    let root = decl(row_layout(12), [30.0, 30.0, 30.0, 255.0]);
    clay.ui(id("root", 0), &root, || {
        // Transparent with a border: no rectangle, one border command.
        let mut b = decl(row_layout(0), [0.0, 0.0, 0.0, 0.0]);
        b.layout.sizing.width = sizing_fixed(100.0);
        b.layout.sizing.height = sizing_fixed(60.0);
        b.border = sys::BorderElementConfig {
            color: [255.0, 0.0, 0.0, 255.0],
            width: mv_clay::border_outside(2),
        };
        clay.ui(id("bordered", 0), &b, || {});
        // A scroll container: vertical scrolling, no content yet.
        let mut s = decl(row_layout(0), [0.0, 0.0, 0.0, 0.0]);
        s.layout.sizing.width = sizing_fixed(100.0);
        s.layout.sizing.height = sizing_fixed(60.0);
        s.clip = sys::ClipElementConfig {
            horizontal: false,
            vertical: true,
            childOffset: [0.0, 0.0],
        };
        clay.ui(id("scrolled", 0), &s, || {});
        // A custom element with a caller-typed payload.
        let mut c = decl(row_layout(0), [0.0, 0.0, 0.0, 0.0]);
        c.layout.sizing.width = sizing_fixed(100.0);
        c.layout.sizing.height = sizing_fixed(60.0);
        c.custom = sys::CustomElementConfig {
            customData: &payload as *const u64 as *mut core::ffi::c_void,
        };
        c.cornerRadius = mv_clay::corner_radius_all(4.0);
        clay.ui(id("custom", 0), &c, || {});
        let config: sys::TextElementConfig = unsafe { core::mem::zeroed() };
        clay.text("hi", &config);
    });
    let commands = clay.end_layout();

    let rect_cmd = commands
        .iter()
        .find(|c| c.commandType == sys::RenderCommandType::Rectangle)
        .expect("one rectangle command");
    let rect = mv_clay::rect_of(rect_cmd).expect("rectangle payload");
    assert_eq!(rect.backgroundColor, [30.0, 30.0, 30.0, 255.0]);
    assert_eq!(mv_clay::border_of(rect_cmd), None);

    let border_cmd = commands
        .iter()
        .find(|c| c.commandType == sys::RenderCommandType::Border)
        .expect("one border command");
    let border = mv_clay::border_of(border_cmd).expect("border payload");
    assert_eq!(border.width.left, 2);
    assert_eq!(border.color, [255.0, 0.0, 0.0, 255.0]);
    assert_eq!(mv_clay::rect_of(border_cmd), None);

    let (custom, corners) = mv_clay::custom_of::<u64>(
        commands
            .iter()
            .find(|c| c.commandType == sys::RenderCommandType::Custom)
            .expect("one custom command"),
    )
    .expect("custom payload");
    assert_eq!(*custom, 0xDEAD);
    assert_eq!(corners.topLeft, 4.0);
    assert!(MEASURED.load(Ordering::SeqCst));

    // The free functions query the same laid-out frame.
    let root_data = mv_clay::element_data(id("root", 0));
    assert!(root_data.found);
    assert_eq!(
        root_data.boundingBox,
        sys::BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 100.0,
        }
    );
    let scroll_data = mv_clay::scroll_container_data(id("scrolled", 0));
    assert!(scroll_data.found);
    assert!(!mv_clay::scroll_container_data(id("root", 0)).found);

    // `command_at` copies out of a borrowed array.
    let array = sys::RenderCommandArray {
        capacity: commands.len() as i32,
        length: commands.len() as i32,
        internalArray: commands.as_ptr() as *mut sys::RenderCommand,
    };
    let back = mv_clay::command_at(&array, 0);
    assert_eq!(back.commandType, commands[0].commandType);
}

/// `image_of` and `effect_of` on hand-built commands: the layout never makes
/// an image, and effects ride `userData` from the frame-lived store.
#[test]
fn image_and_effect_readers() {
    let rect_cmd = sys::RenderCommand {
        boundingBox: sys::BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        renderData: sys::RenderCommandData {
            rectangle: sys::RectangleRenderData {
                backgroundColor: [1.0, 2.0, 3.0, 255.0],
                cornerRadius: mv_clay::corner_radius_all(0.0),
            },
        },
        userData: core::ptr::null_mut(),
        id: 0,
        zIndex: 0,
        commandType: sys::RenderCommandType::Rectangle,
    };
    assert!(mv_clay::image_of(&rect_cmd).is_none());
    assert_eq!(mv_clay::effect_of::<u64>(&rect_cmd), None);

    let img_cmd = sys::RenderCommand {
        commandType: sys::RenderCommandType::Image,
        renderData: sys::RenderCommandData {
            image: sys::ImageRenderData {
                backgroundColor: [0.0, 0.0, 0.0, 0.0],
                cornerRadius: mv_clay::corner_radius_all(0.0),
                imageData: core::ptr::null_mut(),
            },
        },
        ..rect_cmd
    };
    assert!(mv_clay::image_of(&img_cmd).is_some());

    let value: u64 = 7;
    let fx_cmd = sys::RenderCommand {
        userData: &value as *const u64 as *mut core::ffi::c_void,
        ..rect_cmd
    };
    assert_eq!(mv_clay::effect_of::<u64>(&fx_cmd), Some(7));
}
