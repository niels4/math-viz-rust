//! Safe Clay (`mv-sys::clay`): arena, measure callback, element scopes,
//! text, element data, scroll containers, pointer state, the tests lock.
//!
//! Ported from the Odin app's use of `vendor/clay/clay.odin`: every `extern`
//! item is re-exported from `mv-sys`, and this crate adds the `Clay` owner
//! plus the small constructor helpers (`PaddingAll`, `SizingFit`, …) the
//! layout code calls. Method bodies land in `t0c-a`/`t0c-b`; this surface
//! only fixes the signatures. Every public item names its Odin declaration.
//!
//! `unsafe` lives in `mv-sys` only: all public API here is safe. The C
//! library keeps one process-wide context, so layout tests serialize on
//! [`tests_lock`] (plan §1.4).

use std::sync::{Mutex, MutexGuard};

use core::ffi::{c_char, c_int};

use mv_sys::clay as sys;

pub use sys::*;

// --- Context ------------------------------------------------------------------

/// The Clay context: owns the arena memory and the error callback.
///
/// Ported from `clay.odin:483` (`Initialize`): the `Arena` it takes is built
/// here from a capacity, and the `ErrorHandler` from a Rust closure.
// The fields keep their allocations alive for the C side (the arena memory,
// the error closure); the C library reads them through raw pointers, which
// the compiler cannot see.
#[allow(dead_code)]
pub struct Clay {
    arena: Vec<u8>,
    context: *mut sys::Context,
    dimensions: sys::Dimensions,
    on_error: Box<dyn Fn(sys::ErrorData) + Send + Sync + 'static>,
}

/// The text-measurement closure, as the C callback receives it.
type MeasureThunk = dyn for<'a, 'b> Fn(&'a sys::StringSlice, &'b sys::TextElementConfig) -> sys::Dimensions
    + Send
    + Sync;

/// Routes Clay's error callback to the closure stored in the context.
unsafe extern "C" fn clay_error_trampoline(data: sys::ErrorData) {
    let holder =
        unsafe { &*(data.userData as *const *const (dyn Fn(sys::ErrorData) + Send + Sync)) };
    let f: &(dyn Fn(sys::ErrorData) + Send + Sync) = unsafe { &**holder };
    f(data);
}

/// Routes Clay's text measurement to the closure leaked for the context.
unsafe extern "C" fn clay_measure_trampoline(
    text: sys::StringSlice,
    config: *mut sys::TextElementConfig,
    user_data: *mut core::ffi::c_void,
) -> sys::Dimensions {
    let holder = unsafe { &*(user_data as *const *const MeasureThunk) };
    let f: &MeasureThunk = unsafe { &**holder };
    f(&text, unsafe { &*config })
}

impl Clay {
    /// Create a Clay context with an arena of `arena_capacity` bytes.
    ///
    /// The arena is at least `Clay_MinMemorySize` bytes even when asked for
    /// less: Clay errors below its minimum.
    /// Ported from `clay.odin:483` (`Initialize`, with `clay.odin:479-480`
    /// `MinMemorySize`/`CreateArenaWithCapacityAndMemory` folded in).
    pub fn new(
        arena_capacity: usize,
        dims: sys::Dimensions,
        on_error: impl Fn(sys::ErrorData) + Send + Sync + 'static,
    ) -> Self {
        // Clay requires at least its minimum; a smaller capacity errors at
        // the first layout.
        let capacity = arena_capacity.max(unsafe { sys::Clay_MinMemorySize() } as usize);
        let mut arena = vec![0u8; capacity];
        let memory = arena.as_mut_ptr();
        // The closure lives in the struct's `on_error`, so the C side keeps
        // no unregister hook: the holder cell leaks with the context. The fat
        // pointer goes into the heap box first (taking `&on_error` would
        // point at this function's dead parameter slot).
        let on_error: Box<dyn Fn(sys::ErrorData) + Send + Sync> = Box::new(on_error);
        let holder: Box<*const (dyn Fn(sys::ErrorData) + Send + Sync)> =
            Box::new(&*on_error as *const (dyn Fn(sys::ErrorData) + Send + Sync));
        let user_data = Box::into_raw(holder) as *mut core::ffi::c_void;
        let handler = sys::ErrorHandler {
            handler: Some(clay_error_trampoline),
            userData: user_data,
        };
        let arena_sys = sys::Arena {
            nextAllocation: 0,
            capacity,
            memory,
        };
        let context = unsafe { sys::Clay_Initialize(arena_sys, dims, handler) };
        Self {
            arena,
            context,
            dimensions: dims,
            on_error,
        }
    }

    /// Install the text-measurement callback Clay calls during layout.
    ///
    /// Ported from `clay.odin:500` (`SetMeasureTextFunction`). The closure
    /// is kept alive for the context's lifetime; the trampoline bridging it
    /// to the C callback lands with the body.
    pub fn set_measure_text(
        &self,
        measure: impl Fn(&sys::StringSlice, &sys::TextElementConfig) -> sys::Dimensions
        + Send
        + Sync
        + 'static,
    ) {
        // The closure has no owner to move into (`&self` only), so it leaks:
        // both the closure and the holder cell stay alive for the C side,
        // which keeps no unregister hook (16 bytes per call plus the closure).
        let leaked: &'static MeasureThunk = Box::leak(Box::new(measure));
        let holder: Box<*const MeasureThunk> = Box::new(leaked as *const MeasureThunk);
        let user_data = Box::into_raw(holder) as *mut core::ffi::c_void;
        unsafe { sys::Clay_SetMeasureTextFunction(Some(clay_measure_trampoline), user_data) }
    }

    /// Open a layout pass.
    ///
    /// Ported from `clay.odin:488` (`BeginLayout`). Clears the frame text
    /// store: the previous frame drew its Text commands already, and every
    /// text it declares is retained anew below.
    pub fn begin_layout(&mut self) {
        TEXTS.with(|store| store.borrow_mut().clear());
        unsafe { sys::Clay_BeginLayout() }
    }

    /// Close the layout pass and borrow its render commands.
    ///
    /// Ported from `clay.odin:489` (`EndLayout`).
    pub fn end_layout(&mut self) -> &[sys::RenderCommand] {
        let array = unsafe { sys::Clay_EndLayout(0.0) };
        let len = array.length as usize;
        unsafe {
            core::slice::from_raw_parts(array.internalArray as *const sys::RenderCommand, len)
        }
    }

    /// Open an element, configure it, run the children, close it.
    ///
    /// Ported from `clay.odin:528-542` (`UI_WithId`/`UI_AutoId` with
    /// `ConfigureOpenElement`): the `open → configure → close` scope the
    /// Odin `UI()()` expresses with `defer`.
    pub fn ui(
        &self,
        id: sys::ElementId,
        config: &sys::ElementDeclaration,
        children: impl FnOnce(),
    ) {
        unsafe { sys::Clay_OpenElementWithId(id) };
        unsafe { sys::Clay_ConfigureOpenElement(*config) };
        children();
        unsafe { sys::Clay_CloseElement() };
    }

    /// Add a text element with an already-measured config.
    ///
    /// Ported from `clay.odin:544-557` (`Text`/`TextStatic`/`TextDynamic`).
    /// The bytes are copied into the frame text store and Clay gets the
    /// stored pointer: Clay keeps the pointer (re-measuring it at
    /// `EndLayout`, `clay.h:2589`) while the caller's `&str` — a cased
    /// copy, a formatted line — dies at the end of the declare call. The
    /// Odin app keeps such text in the frame's temp allocator
    /// (`typography.odin:170`); the store is this port's temp allocator,
    /// cleared at the next `begin_layout`, after the frame drew.
    pub fn text(&self, text: &str, config: &sys::TextElementConfig) {
        let (chars, length) = TEXTS.with(|store| {
            let mut store = store.borrow_mut();
            store.push(text.as_bytes().to_vec());
            let last = store.last().expect("text just pushed");
            (last.as_ptr(), last.len())
        });
        unsafe {
            sys::Clay_OpenTextElement(
                sys::String {
                    isStaticallyAllocated: false,
                    length: length as c_int,
                    chars: chars as *mut c_char,
                },
                *config,
            );
        }
    }

    /// Whether the pointer is over any element this frame.
    ///
    /// Ported from `clay.odin:494` (`Hovered`).
    pub fn hovered(&self) -> bool {
        unsafe { sys::Clay_Hovered() }
    }

    /// Whether the pointer is over the element with `id`.
    ///
    /// Ported from `clay.odin:496` (`PointerOver`).
    pub fn pointer_over(&self, id: sys::ElementId) -> bool {
        unsafe { sys::Clay_PointerOver(id) }
    }

    /// This frame's layout data for the element with `id`.
    ///
    /// Ported from `clay.odin:493` (`GetElementData`).
    pub fn get_element_data(&self, id: sys::ElementId) -> sys::ElementData {
        unsafe { sys::Clay_GetElementData(id) }
    }

    /// This frame's scroll-container data for the element with `id`.
    pub fn get_scroll_container_data(&self, id: sys::ElementId) -> sys::ScrollContainerData {
        unsafe { sys::Clay_GetScrollContainerData(id) }
    }

    /// Feed the pointer state for the next layout.
    ///
    /// Ported from `clay.odin:481` (`SetPointerState`).
    pub fn set_pointer_state(&mut self, position: sys::Vector2, pointer_down: bool) {
        unsafe { sys::Clay_SetPointerState(position, pointer_down) }
    }

    /// Advance scroll containers by a frame's input.
    ///
    /// Ported from `clay.odin:486` (`UpdateScrollContainers`).
    pub fn update_scroll_containers(
        &mut self,
        enable_drag_scrolling: bool,
        scroll_delta: sys::Vector2,
        delta_time: f32,
    ) {
        unsafe { sys::Clay_UpdateScrollContainers(enable_drag_scrolling, scroll_delta, delta_time) }
    }

    /// The main scroll offset.
    ///
    /// Ported from `clay.odin:498` (`GetScrollOffset`).
    pub fn get_scroll_offset(&self) -> sys::Vector2 {
        unsafe { sys::Clay_GetScrollOffset() }
    }

    /// Resize the layout dimensions (window resize).
    ///
    /// Ported from `clay.odin:487` (`SetLayoutDimensions`).
    pub fn set_layout_dimensions(&mut self, dimensions: sys::Dimensions) {
        self.dimensions = dimensions;
        unsafe { sys::Clay_SetLayoutDimensions(dimensions) }
    }

    /// Toggle Clay's debug view.
    ///
    /// Ported from `clay.odin:503` (`SetDebugModeEnabled`).
    pub fn set_debug_mode(&mut self, enabled: bool) {
        unsafe { sys::Clay_SetDebugModeEnabled(enabled) }
    }
}

// `Clay` owns a raw context pointer and pinned-down callbacks; it stays
// `!Send`/`!Sync` (the raw pointer opts it out) until the body proves the
// threading story.

// The frame's text bytes: `Clay::text` copies every declared string here
// and hands Clay the stored pointer, because Clay keeps the pointer
// (re-measuring it at `EndLayout`) while Rust temporaries die at the end of
// the declare call. Each push keeps its own heap buffer, so growing the
// outer `Vec` never moves a buffer an earlier element points at (the same
// shape as the page's frame-lived `CUSTOMS` in `widgets.rs`); cleared at
// the next `begin_layout`, after the frame drew its Text commands.
std::thread_local! {
    static TEXTS: std::cell::RefCell<Vec<Vec<u8>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

impl Drop for Clay {
    /// Frees the arena; if this context is current, clear it.
    ///
    /// The C context lives inside the arena this struct owns
    /// (`Clay__Context_Allocate_Arena`), and `Clay_Initialize` reads the
    /// previous current context for its defaults. Dropping without clearing
    /// would leave the next `Clay::new` reading freed memory. A shadowed
    /// instance (a later `Clay::new` made another current) leaves the
    /// current context alone.
    fn drop(&mut self) {
        unsafe {
            if sys::Clay_GetCurrentContext() == self.context {
                sys::Clay_SetCurrentContext(core::ptr::null_mut());
            }
        }
    }
}

// --- Tests lock ------------------------------------------------------------------

/// Serializes layout tests: the C library keeps one process-wide context.
///
/// Ported from the plan (§1.4, the Odin run's `088a22c`): every test that
/// lays out holds this guard for its whole body, because `cargo test` runs
/// tests on threads.
pub fn tests_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// --- Constructor helpers ------------------------------------------------------------

/// Padding with the same value on all four sides.
///
/// Ported from `clay.odin:559` (`PaddingAll`).
pub fn padding_all(all_padding: u16) -> sys::Padding {
    sys::Padding {
        left: all_padding,
        right: all_padding,
        top: all_padding,
        bottom: all_padding,
    }
}

/// A border width on the four outer sides.
///
/// Ported from `clay.odin:563` (`BorderOutside`).
pub fn border_outside(width: u16) -> sys::BorderWidth {
    sys::BorderWidth {
        left: width,
        right: width,
        top: width,
        bottom: width,
        betweenChildren: 0,
    }
}

/// A border width on all sides including between children.
///
/// Ported from `clay.odin:567` (`BorderAll`).
pub fn border_all(width: u16) -> sys::BorderWidth {
    sys::BorderWidth {
        left: width,
        right: width,
        top: width,
        bottom: width,
        betweenChildren: width,
    }
}

/// A corner radius on all four corners.
///
/// Ported from `clay.odin:571` (`CornerRadiusAll`).
pub fn corner_radius_all(radius: f32) -> sys::CornerRadius {
    sys::CornerRadius {
        topLeft: radius,
        topRight: radius,
        bottomLeft: radius,
        bottomRight: radius,
    }
}

/// A fit sizing axis with optional min/max.
///
/// Ported from `clay.odin:575` (`SizingFit`).
pub fn sizing_fit(size_min_max: sys::SizingConstraintsMinMax) -> sys::SizingAxis {
    sys::SizingAxis {
        constraints: sys::SizingConstraints {
            sizeMinMax: size_min_max,
        },
        r#type: sys::SizingType::Fit,
    }
}

/// A grow sizing axis with optional min/max.
///
/// Ported from `clay.odin:579` (`SizingGrow`).
pub fn sizing_grow(size_min_max: sys::SizingConstraintsMinMax) -> sys::SizingAxis {
    sys::SizingAxis {
        constraints: sys::SizingConstraints {
            sizeMinMax: size_min_max,
        },
        r#type: sys::SizingType::Grow,
    }
}

/// A fixed sizing axis of `size` pixels.
///
/// Ported from `clay.odin:583` (`SizingFixed`).
pub fn sizing_fixed(size: f32) -> sys::SizingAxis {
    let min_max = sys::SizingConstraintsMinMax {
        min: size,
        max: size,
    };
    sys::SizingAxis {
        constraints: sys::SizingConstraints {
            sizeMinMax: min_max,
        },
        r#type: sys::SizingType::Fixed,
    }
}

/// A percent sizing axis.
///
/// Ported from `clay.odin:587` (`SizingPercent`).
pub fn sizing_percent(size_percent: f32) -> sys::SizingAxis {
    let min_max = sys::SizingConstraintsMinMax {
        min: size_percent,
        max: size_percent,
    };
    sys::SizingAxis {
        constraints: sys::SizingConstraints {
            sizeMinMax: min_max,
        },
        r#type: sys::SizingType::Percent,
    }
}

/// Wrap a Rust string for Clay without copying.
///
/// Ported from `clay.odin:591` (`MakeString`). The returned `String` borrows
/// `label`: keep `label` alive until the layout pass ends.
pub fn make_string(label: &str) -> sys::String {
    sys::String {
        isStaticallyAllocated: false,
        length: label.len() as c_int,
        chars: label.as_ptr() as *mut c_char,
    }
}

/// Hash an element id from a label.
///
/// Ported from `clay.odin:595` (`ID`).
pub fn id(label: &str, index: u32) -> sys::ElementId {
    unsafe { sys::Clay_HashStringWithOffset(make_string(label), index, 0) }
}

/// Hash an element id scoped to the open element.
///
/// Ported from `clay.odin:599` (`ID_LOCAL`).
pub fn id_local(label: &str, index: u32) -> sys::ElementId {
    let seed = unsafe { sys::Clay_GetOpenElementId() };
    unsafe { sys::Clay_HashStringWithOffset(make_string(label), index, seed) }
}

// --- Element scopes without an owner --------------------------------------------------

// The page's declare paths (`widgets.rs`, the overlay bridges) take no `App`
// and hold no `&Clay`, so they cannot reach the methods above; the C library
// keeps one process-wide current context (set by `Clay::new`), and these free
// functions operate on it. The invariant is the Odin one: a layout must be
// open (`begin_layout` ran and `end_layout` has not), as `UI()(){}` shows at
// `clay.odin:527-536`.

/// Open the element `id`: the `UI(id)` half that `configure_open_element`
/// finishes.
///
/// Ported from `clay.odin:477` (`_OpenElementWithId`).
pub fn open_element(id: sys::ElementId) {
    unsafe { sys::Clay_OpenElementWithId(id) }
}

/// Open an element with an automatic id: the `UI()` half.
///
/// Ported from `clay.odin:476` (`_OpenElement`).
pub fn open_element_auto() {
    unsafe { sys::Clay_OpenElement() }
}

/// Configure the open element: the second half of the `UI()(){}` scope.
///
/// Ported from `clay.odin:516` (`_ConfigureOpenElement`).
pub fn configure_open_element(config: &sys::ElementDeclaration) {
    unsafe { sys::Clay_ConfigureOpenElement(*config) }
}

/// Close the open element: the scope's end (Odin's `defer`).
///
/// Ported from `clay.odin:478` (`_CloseElement`).
pub fn close_element() {
    unsafe { sys::Clay_CloseElement() }
}

/// This frame's layout data for the element with `id`.
///
/// Ported from `clay.odin:493` (`GetElementData`).
pub fn element_data(id: sys::ElementId) -> sys::ElementData {
    unsafe { sys::Clay_GetElementData(id) }
}

/// This frame's scroll-container data for the element with `id`.
///
/// Ported from `clay.odin:499` (`GetScrollContainerData`).
pub fn scroll_container_data(id: sys::ElementId) -> sys::ScrollContainerData {
    unsafe { sys::Clay_GetScrollContainerData(id) }
}

// --- Render-command data ----------------------------------------------------------

// The draw walk (`draw.odin:89` `draw_commands`) reads each command's
// payload out of `renderData`'s union and each effect out of `userData`;
// both need `unsafe`, so the copies and borrows live here and draw code
// stays safe. Payload types it cannot name (the page's `Effect` and
// `Custom`) arrive as type parameters, filled by the caller.

/// A Rectangle command's payload; `None` for other commands.
pub fn rect_of(command: &sys::RenderCommand) -> Option<sys::RectangleRenderData> {
    if command.commandType == sys::RenderCommandType::Rectangle {
        Some(unsafe { command.renderData.rectangle })
    } else {
        None
    }
}

/// A Border command's payload; `None` for other commands.
pub fn border_of(command: &sys::RenderCommand) -> Option<sys::BorderRenderData> {
    if command.commandType == sys::RenderCommandType::Border {
        Some(unsafe { command.renderData.border })
    } else {
        None
    }
}

/// An Image command's payload; `None` for other commands.
pub fn image_of(command: &sys::RenderCommand) -> Option<sys::ImageRenderData> {
    if command.commandType == sys::RenderCommandType::Image {
        Some(unsafe { command.renderData.image })
    } else {
        None
    }
}

/// A Text command's payload; `None` for other commands.
pub fn text_of(command: &sys::RenderCommand) -> Option<sys::TextRenderData> {
    if command.commandType == sys::RenderCommandType::Text {
        Some(unsafe { command.renderData.text })
    } else {
        None
    }
}

/// A command's user data pointer, if any.
pub fn user_data_of(command: &sys::RenderCommand) -> *mut core::ffi::c_void {
    command.userData
}

/// A Custom command's payload of type `T` and its corner radius; `None`
/// for other commands or a null `customData`.
///
/// The page boxes its `Custom` payloads for the frame (`widgets.odin:61`
/// `custom_data`, as `customData`); the borrow keeps no frame state.
pub fn custom_of<T>(command: &sys::RenderCommand) -> Option<(&T, sys::CornerRadius)> {
    if command.commandType == sys::RenderCommandType::Custom {
        let data = unsafe { &command.renderData.custom };
        let payload = data.customData as *const T;
        if payload.is_null() {
            None
        } else {
            Some((unsafe { &*payload }, data.cornerRadius))
        }
    } else {
        None
    }
}

/// The `T`-shaped value behind a command's `userData`, copied out; `None`
/// for a null pointer.
///
/// The page packs its `Effect` payloads for the frame (`effects.odin:24`
/// `effect`, as `userData`, read back in `draw.odin:106,147`); the copy
/// keeps no borrow of the frame-lived store.
pub fn effect_of<T: Copy>(command: &sys::RenderCommand) -> Option<T> {
    let payload = command.userData as *const T;
    if payload.is_null() {
        None
    } else {
        Some(unsafe { *payload })
    }
}

/// The `index`th render command, copied out.
///
/// Ported from `clay.odin:502` (`RenderCommandArray_Get`): out of range
/// reads the array's default command, as the C side does. The draw walk
/// builds its array over the laid-out frame (`draw.odin:40` `draw_frame`).
///
/// An out-of-range index fires Clay's error handler through the current
/// context (`clay.h:3998-4004` `Clay__Array_RangeCheck`), and segfaults when
/// no context is live: only call this against a laid-out frame.
pub fn command_at(commands: &sys::RenderCommandArray, index: i32) -> sys::RenderCommand {
    unsafe { *sys::Clay_RenderCommandArray_Get(commands as *const _ as *mut _, index) }
}
