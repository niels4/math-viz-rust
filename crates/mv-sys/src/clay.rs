//! Clay `e6cc369` (`vendor/clay/clay.odin`, all of it).
//!
//! `extern` symbols carry the `Clay_` prefix the Odin binding declares via
//! `link_prefix`; types are `#[repr(C)]` in Odin/C field order. Backings
//! follow `clay.h` on the port's targets (macOS, emscripten): packed enums
//! are one byte (`CLAY_PACKED_ENUM`), `TransitionState` and
//! `TransitionProperty` are plain C enums (`c_int`). On Windows MSVC the
//! Odin binding widens packed enums to `u32`; that target keeps the Odin
//! binding's `EnumBackingType` and is `t0c-a`/`t0c-b`'s call to confirm.

#![allow(non_camel_case_types, non_snake_case)]

use core::ffi::{c_char, c_float, c_int, c_void};

// --- Strings, vectors, arena -----------------------------------------------

/// Ported from `clay.odin:19` (`String`).
///
/// No `PartialEq`: `chars` is a raw pointer.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct String {
    pub isStaticallyAllocated: bool,
    pub length: c_int,
    pub chars: *mut c_char,
}

/// Ported from `clay.odin:25` (`StringSlice`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StringSlice {
    pub length: c_int,
    pub chars: *mut c_char,
    pub baseChars: *mut c_char,
}

impl StringSlice {
    /// The slice as a string. Clay text the app measures comes from Rust
    /// strings, so this is always valid UTF-8 in practice.
    pub fn as_str(&self) -> &str {
        let bytes =
            unsafe { core::slice::from_raw_parts(self.chars as *const u8, self.length as usize) };
        core::str::from_utf8(bytes).expect("Clay text must be UTF-8")
    }
}

/// Ported from `clay.odin:31` (`Vector2`).
pub type Vector2 = [c_float; 2];

/// Ported from `clay.odin:74` (`EnumBackingType`).
///
/// The packed enums' backing on the port's targets (macOS, emscripten);
/// keep in sync with `clay.h`'s `CLAY_PACKED_ENUM`. On Windows MSVC the
/// Odin binding widens this to `u32`; that target is `t0c-a`/`t0c-b`'s call.
pub type EnumBackingType = u8;

/// Ported from `clay.odin:33` (`Dimensions`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dimensions {
    pub width: c_float,
    pub height: c_float,
}

/// Ported from `clay.odin:38` (`Arena`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Arena {
    pub nextAllocation: usize,
    pub capacity: usize,
    pub memory: *mut u8,
}

/// Ported from `clay.odin:44` (`BoundingBox`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundingBox {
    pub x: c_float,
    pub y: c_float,
    pub width: c_float,
    pub height: c_float,
}

/// Ported from `clay.odin:51` (`Color`).
pub type Color = [c_float; 4];

/// Ported from `clay.odin:53` (`CornerRadius`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerRadius {
    pub topLeft: c_float,
    pub topRight: c_float,
    pub bottomLeft: c_float,
    pub bottomRight: c_float,
}

/// Ported from `clay.odin:60` (`ElementId`).
///
/// No `PartialEq`: `stringId.chars` is a raw pointer.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ElementId {
    pub id: u32,
    pub offset: u32,
    pub baseId: u32,
    pub stringId: String,
}

/// Ported from `clay.odin:67` (`ElementIdArray`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ElementIdArray {
    pub capacity: i32,
    pub length: i32,
    pub internalArray: *mut ElementId,
}

// --- Element config enums ----------------------------------------------------

/// Ported from `clay.odin:79` (`RenderCommandType`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderCommandType {
    None = 0,
    Rectangle = 1,
    Border = 2,
    Text = 3,
    Image = 4,
    ScissorStart = 5,
    ScissorEnd = 6,
    OverlayColorStart = 7,
    OverlayColorEnd = 8,
    Custom = 9,
}

/// Ported from `clay.odin:92` (`RectangleElementConfig`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectangleElementConfig {
    pub color: Color,
}

/// Ported from `clay.odin:96` (`TextWrapMode`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextWrapMode {
    Words = 0,
    Newlines = 1,
    None = 2,
}

/// Ported from `clay.odin:102` (`TextAlignment`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextAlignment {
    Left = 0,
    Center = 1,
    Right = 2,
}

/// Ported from `clay.odin:108` (`TextElementConfig`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TextElementConfig {
    pub userData: *mut c_void,
    pub textColor: Color,
    pub fontId: u16,
    pub fontSize: u16,
    pub letterSpacing: u16,
    pub lineHeight: u16,
    pub wrapMode: TextWrapMode,
    pub textAlignment: TextAlignment,
}

/// Ported from `clay.odin:119` (`AspectRatioElementConfig`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AspectRatioElementConfig {
    pub aspectRatio: f32,
}

/// Ported from `clay.odin:123` (`ImageElementConfig`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ImageElementConfig {
    pub imageData: *mut c_void,
}

/// Ported from `clay.odin:127` (`CustomElementConfig`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CustomElementConfig {
    pub customData: *mut c_void,
}

/// Ported from `clay.odin:131` (`BorderWidth`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderWidth {
    pub left: u16,
    pub right: u16,
    pub top: u16,
    pub bottom: u16,
    pub betweenChildren: u16,
}

/// Ported from `clay.odin:139` (`BorderElementConfig`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderElementConfig {
    pub color: Color,
    pub width: BorderWidth,
}

// --- Transitions ---------------------------------------------------------------

/// Ported from `clay.odin:144` (`TransitionData`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransitionData {
    pub boundingBox: BoundingBox,
    pub backgroundColor: Color,
    pub overlayColor: Color,
    pub borderColor: Color,
    pub borderWidth: BorderWidth,
}

/// Ported from `clay.odin:152` (`TransitionState`; plain C enum).
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransitionState {
    Idle = 0,
    Entering = 1,
    Transitioning = 2,
    Exiting = 3,
}

/// Ported from `clay.odin:159` (`TransitionProperty`).
///
/// Values are the C header's (`CLAY_TRANSITION_PROPERTY_*`); the Odin
/// binding's `TransitionPropertyFlags` bit-set combinations become the
/// `TRANSITION_PROPERTY_*` constants below.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransitionProperty {
    X = 1,
    Y = 2,
    Width = 4,
    Height = 8,
    BackgroundColor = 16,
    OverlayColor = 32,
    CornerRadius = 64,
    BorderColor = 128,
    BorderWidth = 256,
}

/// The C `Clay_TransitionProperty` bitmask as passed across the FFI.
///
/// Ported from `clay.odin:171` (`TransitionPropertyFlags`).
pub type TransitionPropertyFlags = c_int;

/// Ported from `clay.odin:172` (`TransitionPropertyPosition`).
pub const TRANSITION_PROPERTY_POSITION: TransitionPropertyFlags = 1 | 2;
/// Ported from `clay.odin:173` (`TransitionPropertyDimensions`).
pub const TRANSITION_PROPERTY_DIMENSIONS: TransitionPropertyFlags = 4 | 8;
/// Ported from `clay.odin:174` (`TransitionPropertyBoundingBox`).
pub const TRANSITION_PROPERTY_BOUNDING_BOX: TransitionPropertyFlags = 1 | 2 | 4 | 8;
/// Ported from `clay.odin:175` (`TransitionPropertyBorder`).
pub const TRANSITION_PROPERTY_BORDER: TransitionPropertyFlags = 128 | 256;

/// Ported from `clay.odin:177` (`TransitionCallbackArguments`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TransitionCallbackArguments {
    pub transitionState: TransitionState,
    pub initial: TransitionData,
    pub current: *mut TransitionData,
    pub target: TransitionData,
    pub elapsedTime: c_float,
    pub duration: c_float,
    pub properties: TransitionPropertyFlags,
}

/// Ported from `clay.odin:187` (`TransitionEnterTriggerType`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransitionEnterTriggerType {
    SkipOnFirstParentFrame = 0,
    TriggerOnFirstParentFrame = 1,
}

/// Ported from `clay.odin:192` (`TransitionExitTriggerType`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransitionExitTriggerType {
    SkipWhenParentExits = 0,
    TriggerWhenParentExits = 1,
}

/// Ported from `clay.odin:197` (`TransitionInteractionHandlingType`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransitionInteractionHandlingType {
    DisableInteractionsWhileTransitioningPosition = 0,
    AllowInteractionsWhileTransitioningPosition = 1,
}

/// Ported from `clay.odin:202` (`ExitTransitionSiblingOrdering`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExitTransitionSiblingOrdering {
    UnderneathSiblings = 0,
    NaturalOrder = 1,
    AboveSiblings = 2,
}

/// The Odin binding inlines this struct in `TransitionElementConfig`;
/// named here because C/Rust need a name. From `clay.odin:213-216`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TransitionEnterConfig {
    pub setInitialState:
        Option<unsafe extern "C" fn(TransitionData, TransitionPropertyFlags) -> TransitionData>,
    pub trigger: TransitionEnterTriggerType,
}

/// The Odin binding inlines this struct in `TransitionElementConfig`;
/// named here because C/Rust need a name. From `clay.odin:217-222`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TransitionExitConfig {
    pub setFinalState:
        Option<unsafe extern "C" fn(TransitionData, TransitionPropertyFlags) -> TransitionData>,
    pub trigger: TransitionExitTriggerType,
    pub siblingOrdering: ExitTransitionSiblingOrdering,
}

/// Ported from `clay.odin:208` (`TransitionElementConfig`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TransitionElementConfig {
    pub handler: Option<unsafe extern "C" fn(TransitionCallbackArguments) -> bool>,
    pub duration: c_float,
    pub properties: TransitionPropertyFlags,
    pub interactionHandling: TransitionInteractionHandlingType,
    pub enter: TransitionEnterConfig,
    pub exit: TransitionExitConfig,
}

// --- Floating, clip, render data -----------------------------------------------

/// Ported from `clay.odin:224` (`ClipElementConfig`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipElementConfig {
    pub horizontal: bool,
    pub vertical: bool,
    pub childOffset: Vector2,
}

/// Ported from `clay.odin:230` (`FloatingAttachPointType`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatingAttachPointType {
    LeftTop = 0,
    LeftCenter = 1,
    LeftBottom = 2,
    CenterTop = 3,
    CenterCenter = 4,
    CenterBottom = 5,
    RightTop = 6,
    RightCenter = 7,
    RightBottom = 8,
}

/// Ported from `clay.odin:242` (`FloatingAttachPoints`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatingAttachPoints {
    pub element: FloatingAttachPointType,
    pub parent: FloatingAttachPointType,
}

/// Ported from `clay.odin:247` (`PointerCaptureMode`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerCaptureMode {
    Capture = 0,
    Passthrough = 1,
}

/// Ported from `clay.odin:252` (`FloatingAttachToElement`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatingAttachToElement {
    None = 0,
    Parent = 1,
    ElementWithId = 2,
    Root = 3,
}

/// Ported from `clay.odin:259` (`FloatingClipToElement`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatingClipToElement {
    None = 0,
    AttachedParent = 1,
}

/// Ported from `clay.odin:264` (`FloatingElementConfig`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatingElementConfig {
    pub offset: Vector2,
    pub expand: Dimensions,
    pub parentId: u32,
    pub zIndex: i16,
    pub attachment: FloatingAttachPoints,
    pub pointerCaptureMode: PointerCaptureMode,
    pub attachTo: FloatingAttachToElement,
    pub clipTo: FloatingClipToElement,
}

/// Ported from `clay.odin:275` (`TextRenderData`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TextRenderData {
    pub stringContents: StringSlice,
    pub textColor: Color,
    pub fontId: u16,
    pub fontSize: u16,
    pub letterSpacing: u16,
    pub lineHeight: u16,
}

/// Ported from `clay.odin:284` (`RectangleRenderData`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectangleRenderData {
    pub backgroundColor: Color,
    pub cornerRadius: CornerRadius,
}

/// Ported from `clay.odin:289` (`ImageRenderData`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ImageRenderData {
    pub backgroundColor: Color,
    pub cornerRadius: CornerRadius,
    pub imageData: *mut c_void,
}

/// Ported from `clay.odin:295` (`CustomRenderData`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CustomRenderData {
    pub backgroundColor: Color,
    pub cornerRadius: CornerRadius,
    pub customData: *mut c_void,
}

/// Ported from `clay.odin:301` (`ClipRenderData`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipRenderData {
    pub horizontal: bool,
    pub vertical: bool,
}

/// Ported from `clay.odin:306` (`OverlayColorRenderData`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayColorRenderData {
    pub color: Color,
}

/// Ported from `clay.odin:310` (`BorderRenderData`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderRenderData {
    pub color: Color,
    pub cornerRadius: CornerRadius,
    pub width: BorderWidth,
}

/// Ported from `clay.odin:316` (`RenderCommandData`, `#raw_union`).
///
/// Unions have no `Debug`/`PartialEq`; match on `commandType` first.
#[repr(C)]
#[derive(Clone, Copy)]
pub union RenderCommandData {
    pub rectangle: RectangleRenderData,
    pub text: TextRenderData,
    pub image: ImageRenderData,
    pub custom: CustomRenderData,
    pub border: BorderRenderData,
    pub clip: ClipRenderData,
    pub overlayColor: OverlayColorRenderData,
}

/// Ported from `clay.odin:326` (`RenderCommand`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RenderCommand {
    pub boundingBox: BoundingBox,
    pub renderData: RenderCommandData,
    pub userData: *mut c_void,
    pub id: u32,
    pub zIndex: i16,
    pub commandType: RenderCommandType,
}

/// Ported from `clay.odin:335` (`ScrollContainerData`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ScrollContainerData {
    pub scrollPosition: *mut Vector2,
    pub scrollContainerDimensions: Dimensions,
    pub contentDimensions: Dimensions,
    pub config: ClipElementConfig,
    pub found: bool,
}

/// Ported from `clay.odin:346` (`ElementData`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElementData {
    pub boundingBox: BoundingBox,
    pub found: bool,
}

/// Ported from `clay.odin:351` (`PointerDataInteractionState`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerDataInteractionState {
    PressedThisFrame = 0,
    Pressed = 1,
    ReleasedThisFrame = 2,
    Released = 3,
}

/// Ported from `clay.odin:358` (`PointerData`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerData {
    pub position: Vector2,
    pub state: PointerDataInteractionState,
}

// --- Sizing, layout, declaration -----------------------------------------------

/// Ported from `clay.odin:363` (`SizingType`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SizingType {
    Fit = 0,
    Grow = 1,
    Percent = 2,
    Fixed = 3,
}

/// Ported from `clay.odin:370` (`SizingConstraintsMinMax`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SizingConstraintsMinMax {
    pub min: c_float,
    pub max: c_float,
}

/// Ported from `clay.odin:375` (`SizingConstraints`, `#raw_union`).
#[repr(C)]
#[derive(Clone, Copy)]
pub union SizingConstraints {
    pub sizeMinMax: SizingConstraintsMinMax,
    pub sizePercent: c_float,
}

/// Ported from `clay.odin:380` (`SizingAxis`).
///
/// No `Debug`: `constraints` is a union.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SizingAxis {
    pub constraints: SizingConstraints,
    pub r#type: SizingType,
}

/// Ported from `clay.odin:386` (`Sizing`).
///
/// No `Debug`: `SizingAxis.constraints` is a union.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Sizing {
    pub width: SizingAxis,
    pub height: SizingAxis,
}

/// Ported from `clay.odin:391` (`Padding`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Padding {
    pub left: u16,
    pub right: u16,
    pub top: u16,
    pub bottom: u16,
}

/// Ported from `clay.odin:398` (`LayoutDirection`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayoutDirection {
    LeftToRight = 0,
    TopToBottom = 1,
}

/// Ported from `clay.odin:403` (`LayoutAlignmentX`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayoutAlignmentX {
    Left = 0,
    Right = 1,
    Center = 2,
}

/// Ported from `clay.odin:409` (`LayoutAlignmentY`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayoutAlignmentY {
    Top = 0,
    Bottom = 1,
    Center = 2,
}

/// Ported from `clay.odin:415` (`ChildAlignment`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChildAlignment {
    pub x: LayoutAlignmentX,
    pub y: LayoutAlignmentY,
}

/// Ported from `clay.odin:420` (`LayoutConfig`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LayoutConfig {
    pub sizing: Sizing,
    pub padding: Padding,
    pub childGap: u16,
    pub childAlignment: ChildAlignment,
    pub layoutDirection: LayoutDirection,
}

/// Ported from `clay.odin:428` (`ClayArray`, instantiated only with
/// `RenderCommand` in the binding, so non-generic here).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RenderCommandArray {
    pub capacity: i32,
    pub length: i32,
    pub internalArray: *mut RenderCommand,
}

/// Ported from `clay.odin:434` (`ElementDeclaration`).
///
/// No `Debug`: `layout` holds a union.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElementDeclaration {
    pub layout: LayoutConfig,
    pub backgroundColor: Color,
    pub overlayColor: Color,
    pub cornerRadius: CornerRadius,
    pub aspectRatio: AspectRatioElementConfig,
    pub image: ImageElementConfig,
    pub floating: FloatingElementConfig,
    pub custom: CustomElementConfig,
    pub clip: ClipElementConfig,
    pub border: BorderElementConfig,
    pub transition: TransitionElementConfig,
    pub userData: *mut c_void,
}

/// Ported from `clay.odin:449` (`ErrorType`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorType {
    TextMeasurementFunctionNotProvided = 0,
    ArenaCapacityExceeded = 1,
    ElementsCapacityExceeded = 2,
    TextMeasurementCapacityExceeded = 3,
    DuplicateId = 4,
    FloatingContainerParentNotFound = 5,
    PercentageOver1 = 6,
    InternalError = 7,
    UnbalancedOpenClose = 8,
}

/// Ported from `clay.odin:461` (`ErrorData`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ErrorData {
    pub errorType: ErrorType,
    pub errorText: String,
    pub userData: *mut c_void,
}

impl ErrorData {
    /// The error's message text.
    pub fn message(&self) -> &str {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self.errorText.chars as *const u8,
                self.errorText.length as usize,
            )
        };
        core::str::from_utf8(bytes).expect("Clay error text must be UTF-8")
    }
}

/// Ported from `clay.odin:467` (`ErrorHandler`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ErrorHandler {
    pub handler: Option<unsafe extern "C" fn(ErrorData)>,
    pub userData: *mut c_void,
}

/// Ported from `clay.odin:472` (`Context`, opaque).
#[repr(C)]
#[derive(Debug)]
pub struct Context {
    _private: [u8; 0],
}

// --- The foreign block --------------------------------------------------------

// `improper_ctypes` fires on the by-value `Vector2` (`[f32; 2]`) params and
// returns. C Clay really passes `Clay_Vector2` by value, and on both of this
// port's targets the shapes agree (homogeneous float aggregates: V registers
// on AAPCS64, indirect on wasm32), as for `nvg::Color`.
#[allow(improper_ctypes)]
unsafe extern "C" {
    /// Ported from `clay.odin:476` (`_OpenElement`).
    #[link_name = "Clay__OpenElement"]
    pub unsafe fn Clay_OpenElement();
    /// Ported from `clay.odin:477` (`_OpenElementWithId`).
    #[link_name = "Clay__OpenElementWithId"]
    pub unsafe fn Clay_OpenElementWithId(id: ElementId);
    /// Ported from `clay.odin:478` (`_CloseElement`).
    #[link_name = "Clay__CloseElement"]
    pub unsafe fn Clay_CloseElement();
    /// Ported from `clay.odin:479` (`MinMemorySize`).
    pub unsafe fn Clay_MinMemorySize() -> u32;
    /// Ported from `clay.odin:480` (`CreateArenaWithCapacityAndMemory`).
    pub unsafe fn Clay_CreateArenaWithCapacityAndMemory(capacity: usize, offset: *mut u8) -> Arena;
    /// Ported from `clay.odin:481` (`SetPointerState`).
    pub unsafe fn Clay_SetPointerState(position: Vector2, pointerDown: bool);
    /// Ported from `clay.odin:482` (`GetPointerState`).
    pub unsafe fn Clay_GetPointerState() -> PointerData;
    /// Ported from `clay.odin:483` (`Initialize`).
    pub unsafe fn Clay_Initialize(
        arena: Arena,
        layoutDimensions: Dimensions,
        errorHandler: ErrorHandler,
    ) -> *mut Context;
    /// Ported from `clay.odin:484` (`GetCurrentContext`).
    pub unsafe fn Clay_GetCurrentContext() -> *mut Context;
    /// Ported from `clay.odin:485` (`SetCurrentContext`).
    pub unsafe fn Clay_SetCurrentContext(ctx: *mut Context);
    /// Ported from `clay.odin:486` (`UpdateScrollContainers`).
    pub unsafe fn Clay_UpdateScrollContainers(
        enableDragScrolling: bool,
        scrollDelta: Vector2,
        deltaTime: c_float,
    );
    /// Ported from `clay.odin:487` (`SetLayoutDimensions`).
    pub unsafe fn Clay_SetLayoutDimensions(dimensions: Dimensions);
    /// Ported from `clay.odin:488` (`BeginLayout`).
    pub unsafe fn Clay_BeginLayout();
    /// Ported from `clay.odin:489` (`EndLayout`).
    pub unsafe fn Clay_EndLayout(deltaTime: c_float) -> RenderCommandArray;
    /// Ported from `clay.odin:490` (`GetOpenElementId`).
    pub unsafe fn Clay_GetOpenElementId() -> u32;
    /// Ported from `clay.odin:491` (`GetElementId`).
    pub unsafe fn Clay_GetElementId(id: String) -> ElementId;
    /// Ported from `clay.odin:492` (`GetElementIdWithIndex`).
    pub unsafe fn Clay_GetElementIdWithIndex(id: String, index: u32) -> ElementId;
    /// Ported from `clay.odin:493` (`GetElementData`).
    pub unsafe fn Clay_GetElementData(id: ElementId) -> ElementData;
    /// Ported from `clay.odin:494` (`Hovered`).
    pub unsafe fn Clay_Hovered() -> bool;
    /// Ported from `clay.odin:495` (`OnHover`).
    pub unsafe fn Clay_OnHover(
        onHoverFunction: Option<unsafe extern "C" fn(ElementId, PointerData, *mut c_void)>,
        userData: *mut c_void,
    );
    /// Ported from `clay.odin:496` (`PointerOver`).
    pub unsafe fn Clay_PointerOver(id: ElementId) -> bool;
    /// Ported from `clay.odin:497` (`GetPointerOverIds`).
    pub unsafe fn Clay_GetPointerOverIds() -> ElementIdArray;
    /// Ported from `clay.odin:498` (`GetScrollOffset`).
    pub unsafe fn Clay_GetScrollOffset() -> Vector2;
    /// Ported from `clay.odin:499` (`GetScrollContainerData`).
    pub unsafe fn Clay_GetScrollContainerData(id: ElementId) -> ScrollContainerData;
    /// Ported from `clay.odin:500` (`SetMeasureTextFunction`).
    pub unsafe fn Clay_SetMeasureTextFunction(
        measureTextFunction: Option<
            unsafe extern "C" fn(StringSlice, *mut TextElementConfig, *mut c_void) -> Dimensions,
        >,
        userData: *mut c_void,
    );
    /// Ported from `clay.odin:501` (`SetQueryScrollOffsetFunction`).
    pub unsafe fn Clay_SetQueryScrollOffsetFunction(
        queryScrollOffsetFunction: Option<unsafe extern "C" fn(u32, *mut c_void) -> Vector2>,
        userData: *mut c_void,
    );
    /// Ported from `clay.odin:502` (`RenderCommandArray_Get`).
    pub unsafe fn Clay_RenderCommandArray_Get(
        array: *mut RenderCommandArray,
        index: i32,
    ) -> *mut RenderCommand;
    /// Ported from `clay.odin:503` (`SetDebugModeEnabled`).
    pub unsafe fn Clay_SetDebugModeEnabled(enabled: bool);
    /// Ported from `clay.odin:504` (`IsDebugModeEnabled`).
    pub unsafe fn Clay_IsDebugModeEnabled() -> bool;
    /// Ported from `clay.odin:505` (`SetCullingEnabled`).
    pub unsafe fn Clay_SetCullingEnabled(enabled: bool);
    /// Ported from `clay.odin:506` (`GetMaxElementCount`).
    pub unsafe fn Clay_GetMaxElementCount() -> i32;
    /// Ported from `clay.odin:507` (`SetMaxElementCount`).
    pub unsafe fn Clay_SetMaxElementCount(maxElementCount: i32);
    /// Ported from `clay.odin:508` (`GetMaxMeasureTextCacheWordCount`).
    pub unsafe fn Clay_GetMaxMeasureTextCacheWordCount() -> i32;
    /// Ported from `clay.odin:509` (`SetMaxMeasureTextCacheWordCount`).
    pub unsafe fn Clay_SetMaxMeasureTextCacheWordCount(maxMeasureTextCacheWordCount: i32);
    /// Ported from `clay.odin:510` (`ResetMeasureTextCache`).
    pub unsafe fn Clay_ResetMeasureTextCache();
    /// Ported from `clay.odin:511` (`EaseOut`).
    pub unsafe fn Clay_EaseOut(arguments: TransitionCallbackArguments) -> bool;
    /// Ported from `clay.odin:516` (`_ConfigureOpenElement`).
    #[link_name = "Clay__ConfigureOpenElement"]
    pub unsafe fn Clay_ConfigureOpenElement(config: ElementDeclaration);
    /// Ported from `clay.odin:517` (`_HashString`, backing `ID`).
    #[link_name = "Clay__HashString"]
    pub unsafe fn Clay_HashString(key: String, seed: u32) -> ElementId;
    /// Ported from `clay.odin:518` (`_HashStringWithOffset`, backing `ID_LOCAL`).
    #[link_name = "Clay__HashStringWithOffset"]
    pub unsafe fn Clay_HashStringWithOffset(key: String, index: u32, seed: u32) -> ElementId;
    /// Ported from `clay.odin:519` (`_OpenTextElement`, backing `Text`).
    #[link_name = "Clay__OpenTextElement"]
    pub unsafe fn Clay_OpenTextElement(text: String, textConfig: TextElementConfig);
}
