// The JavaScript side of the foreign procedures in platform_web.rs, linked by
// emcc's --js-library into the "env" module: what SDL's Emscripten port leaves
// to the page (the main loop's pause and resume, the prefs in localStorage,
// prefers-reduced-motion, the canvas's cursor, a trackpad pinch read, and the
// drawing buffer's colour space).
//
// Ported from the Odin app's web/library.js (153 lines): same functions, same
// order, same Emscripten deps. Adapted only in the comments (the storage key
// arrives as an argument, so no key name appears here; the exports are the
// Rust port's `mv_*` procedures) and in `mv_web_init`, which also installs the
// harness's paused-clock step and model-line path the Rust side implements.
addToLibrary({
  // The loop's counts for the harness: Module.iterations (turns of the loop,
  // drawn or not) stands still while the loop is idle, and Module.frames
  // (frames drawn) is what the harness waits on.
  mv_loop_counts: (iterations, frames) => {
    Module.iterations = iterations;
    Module.frames = frames;
  },

  // Idle: the browser's main loop pauses after this turn. Any event SDL
  // queues resumes it (platform_web.rs's event watch); resuming a running
  // loop does nothing.
  mv_loop_idle__deps: ["$MainLoop"],
  mv_loop_idle: () => MainLoop.pause(),
  mv_loop_wake__deps: ["$MainLoop"],
  mv_loop_wake: () => {
    if (MainLoop.func && !MainLoop.scheduler) MainLoop.resume();
  },

  // What SDL's port doesn't read from the page: a trackpad pinch, and
  // prefers-reduced-motion's changes. A trackpad pinch reaches a page as
  // wheel events with ctrlKey set (Chromium sends -100 ln(scale) pixels a
  // step, and a mouse wheel with Ctrl held reads the same way); they are
  // taken here, before SDL's wheel handler on the canvas sees them and
  // before the browser zooms the page, and posted as SDL's own pinch event.
  // A notch is 100 pixels, 3 lines or 1/80 page, as SDL's port counts them.
  mv_web_init__deps: [
    "mv_pinch",
    "mv_hash_changed",
    "mv_reduced_motion_changed",
    "mv_model_line",
    "$stringToNewUTF8",
    "free",
    "$MainLoop",
  ],
  mv_web_init: () => {
    // The harness's input instant: under the paused clock the inputs it has
    // just dispatched take effect at once, as at the rendering update a
    // browser runs after them: one turn of the main loop (its events, a
    // frame), outside its schedule.
    Module.mvIterate = () => {
      if (MainLoop.func) MainLoop.runIter(MainLoop.func);
    };
    // The harness's arming-turn hold (h2-input): after a button or wheel
    // verb, the next turn publishes without stepping the camera, so the
    // gesture's own change draws now and the loop's motion starts next turn
    // — as the reference's next animation frame starts it. Moves never hold.
    Module.mvHoldStep = () => {
      _mv_hold_step();
    };
    // The harness's `port model ...` lines: the port's model set where the
    // port can't yet make the gesture the reference makes. True when the
    // line was taken.
    Module.mvModelLine = (line) => {
      const text = stringToNewUTF8(line);
      try {
        return Boolean(_mv_model_line(text));
      } finally {
        _free(text);
      }
    };
    window.addEventListener(
      "wheel",
      (event) => {
        if (!event.ctrlKey) return;
        event.preventDefault();
        event.stopPropagation();
        const unit = event.deltaMode === 1 ? 100 / 3 : event.deltaMode === 2 ? 8000 : 1;
        _mv_pinch(Math.exp((-event.deltaY * unit) / 100));
      },
      { capture: true, passive: false },
    );
    // The canvas's pointer capture (`useCamera.ts`: the press captures the
    // canvas, released implicitly on pointerup and pointercancel): the
    // capture retargets out-of-element moves to the canvas with working
    // offsets, so a drag past the canvas's edge — and past the viewport's —
    // keeps reaching SDL's canvas listeners as its moves. Without this a
    // handle drag past the viewport stopped following on the web build
    // (h7-fix-capture: web h=10.40 vs native and the reference h=11.44 at
    // x=1500). SDL's port listens on the canvas, so capturing there is what
    // delivers the moves; the hook's try/catch stands, as a mouse already
    // captured (or no pointer support) throws. The hook's two guards stand
    // too (`useCamera.ts:171-176`): no capture with an active pointer, and
    // only button 0 for a mouse. The active id clears on release, cancel
    // and blur, so a lost event cannot stick it and steal a later capture.
    const canvas = Module.canvas ?? document.getElementById("canvas");
    let cameraPointerId = null;
    canvas?.addEventListener("pointerdown", (event) => {
      if (cameraPointerId !== null) return;
      if (event.pointerType === "mouse" && event.button !== 0) return;
      cameraPointerId = event.pointerId;
      try {
        canvas.setPointerCapture(event.pointerId);
      } catch {}
    });
    const releaseCameraPointer = (event) => {
      if (event.pointerId === cameraPointerId) cameraPointerId = null;
    };
    canvas?.addEventListener("pointerup", releaseCameraPointer);
    canvas?.addEventListener("pointercancel", releaseCameraPointer);
    window.addEventListener("blur", () => {
      cameraPointerId = null;
    });
    // Browser shortcuts reach the browser, as on the React page (the user's
    // ruling 2026-10-18; a deliberate departure from the Odin oracle in
    // keyboard default handling only, no pixel effect). SDL's Emscripten port
    // listens in the bubble phase and prevents every key it posts unless text
    // input is active and the key is not an arrow, Backspace, Tab, a function
    // key or a Ctrl combo (SDL_emscriptenevents.c:381-462), so without this
    // every Cmd+key combo would stay with the app. A capture listener on
    // window runs before SDL's, whatever the registration order, and
    // stopPropagation() there keeps the event from SDL, so nothing calls
    // preventDefault and the browser performs its shortcut.
    //
    // The rule table. No Meta held, or the Meta key itself: untouched, so the
    // modifier still reaches SDL (a Cmd-drag reads GUI as held). Meta with
    // Alt: to the browser, field or not. Meta without Alt while no field is
    // edited: to the browser. Meta without Alt while a field is edited: kept
    // with the app, as in a native text field — prevented here, so the
    // browser does nothing with it, and let through, so SDL posts it and
    // field_key acts as now (the in-field cell is the ruling's reading).
    // The keyup (and keypress, where an engine fires one) of a combo whose
    // keydown was stopped is stopped too, so SDL never sees half a key.
    Module.fieldEditing = false;
    const stoppedCodes = new Set();
    const passToBrowser = (event) => {
      // A fresh keydown without Meta ends any stopped combo on this key: the
      // keyup the set waits for never came (the press was interrupted), and
      // the key is down again on its own, so SDL must see its keyup.
      if (event.type === "keydown" && !event.metaKey) stoppedCodes.delete(event.code);
      if (!event.metaKey || event.key === "Meta") return;
      if (event.type === "keydown") {
        if (event.altKey || !Module.fieldEditing) {
          event.stopPropagation();
          stoppedCodes.add(event.code);
          return;
        }
        event.preventDefault();
        return;
      }
      if (stoppedCodes.has(event.code)) {
        event.stopPropagation();
        if (event.type === "keyup") stoppedCodes.delete(event.code);
      }
    };
    window.addEventListener("keydown", passToBrowser, { capture: true });
    window.addEventListener("keyup", passToBrowser, { capture: true });
    window.addEventListener("keypress", passToBrowser, { capture: true });
    matchMedia("(prefers-reduced-motion: reduce)").addEventListener("change", (event) =>
      _mv_reduced_motion_changed(event.matches),
    );
    // The page's hash changing under the app (an edit in the address bar,
    // back/forward): the app re-reads it (mv_hash_changed), as math-viz2's
    // hashchange subscription re-reads its route and search string. Our own
    // writes go through history.replaceState, which fires no hashchange.
    window.addEventListener("hashchange", () => _mv_hash_changed());
  },

  // The colour space the page draws in: Display P3 where the engine shows
  // the reference's colours beyond sRGB (WebKit, Chromium); sRGB in Firefox,
  // which clamps CSS colours to sRGB. The harness's Module.colorSpace
  // (web/index.html), then ?space=p3 or ?space=srgb in the address, override
  // the rule. True for Display P3.
  mv_color_space: () => {
    const asked = Module.colorSpace ?? new URLSearchParams(location.search).get("space");
    if (asked === "p3" || asked === "display-p3") return true;
    if (asked === "srgb") return false;
    return !/\bFirefox\//.test(navigator.userAgent);
  },

  // SDL's WebGL2 context (the canvas hands back the one SDL made) tagged
  // Display P3 or sRGB, from the next frame drawn; whether its drawing
  // buffer is Display P3 after (an engine without the attribute keeps sRGB).
  mv_tag_drawing_buffer: (p3) => {
    const gl = Module.canvas.getContext("webgl2");
    if (!gl || !("drawingBufferColorSpace" in gl)) return false;
    gl.drawingBufferColorSpace = p3 ? "display-p3" : "srgb";
    return gl.drawingBufferColorSpace === "display-p3";
  },

  // ?trace in the page's address logs each fact and timer, as --trace does
  // natively.
  mv_trace: () => new URLSearchParams(location.search).has("trace"),

  // Whether a number field is being edited: the key listeners above keep
  // Meta combos with the app while one is. Set from Rust when a field opens
  // and closes (platform_web.rs's platform_field_editing); the page reads it
  // as a flag, so no export (and no EXPORTED_FUNCTIONS entry) is needed.
  mv_field_editing: (editing) => {
    Module.fieldEditing = editing;
  },

  // ?panic-at-frame in the page's address panics in the first frame, as
  // MV_PANIC_AT_FRAME=1 does natively, so the sweep can prove it sees a
  // crash on the web build.
  mv_panic_at_frame: () => new URLSearchParams(location.search).has("panic-at-frame"),

  mv_reduced_motion: () => matchMedia("(prefers-reduced-motion: reduce)").matches,

  // The canvas's cursor, as a CSS rule: SDL's port writes the canvas's
  // inline cursor whenever the mouse enters it, and an !important rule wins
  // over an inline style.
  mv_set_cursor__deps: ["$UTF8ToString"],
  mv_set_cursor: (name, len) => {
    let style = Module.cursorStyle;
    if (!style) {
      style = Module.cursorStyle = document.createElement("style");
      document.head.appendChild(style);
    }
    style.textContent = `#canvas { cursor: ${UTF8ToString(name, len)} !important; }`;
  },

  // The page's hash query for the URL state: `location.hash` at start
  // (the app cuts the query part itself), written back through
  // `history.replaceState`, so no history entry accrues as math-viz2's
  // debounced URL write lands. Fresh strings cross into wasm, freed there.
  mv_read_hash__deps: ["$stringToNewUTF8"],
  mv_read_hash: () => stringToNewUTF8(location.hash),
  mv_write_hash__deps: ["$UTF8ToString"],
  mv_write_hash: (hash) => {
    history.replaceState(null, "", `#${UTF8ToString(hash)}`);
  },
  // An anchor's navigation (h10-page's welcome button, the wordmark):
  // assigning `location.hash` makes a history entry and fires `hashchange`,
  // which the app's subscription reads back as the address bar does. A
  // function Rust calls, so no export is needed.
  mv_navigate_hash__deps: ["$UTF8ToString"],
  mv_navigate_hash: (hash) => {
    location.hash = UTF8ToString(hash);
  },

  // The prefs in localStorage, one value per key, as math-viz2 stores them
  // (a JSON string per key): the stored text's length in bytes (copied to
  // buf when cap holds it and its terminator), or -1 when nothing is stored
  // or storage throws, as it does in a private window. From the first read
  // of a key on, another tab's write under that key reaches the app
  // (mv_prefs_stored), as a storage subscription takes it: its text, checked
  // there as a read is ("" when the key was removed, which reads as the
  // defaults). Each key is watched on its own first read.
  mv_prefs_load__deps: [
    "$UTF8ToString",
    "$stringToUTF8",
    "$lengthBytesUTF8",
    "$stringToNewUTF8",
    "free",
    "mv_prefs_stored",
  ],
  mv_prefs_load: (key, keyLen, buf, cap) => {
    const name = UTF8ToString(key, keyLen);
    if (!Module.prefsWatched) Module.prefsWatched = {};
    if (!Module.prefsWatched[name]) {
      Module.prefsWatched[name] = true;
      window.addEventListener("storage", (event) => {
        if (event.storageArea !== localStorage || event.key !== name) return;
        const text = stringToNewUTF8(event.newValue ?? "");
        try {
          _mv_prefs_stored(text);
        } finally {
          _free(text);
        }
      });
    }
    let text;
    try {
      text = localStorage.getItem(name);
    } catch {
      return -1;
    }
    if (text === null) return -1;
    const length = lengthBytesUTF8(text);
    if (length < cap) stringToUTF8(text, buf, cap);
    return length;
  },
  mv_prefs_save__deps: ["$UTF8ToString"],
  mv_prefs_save: (key, keyLen, text, len) => {
    try {
      localStorage.setItem(UTF8ToString(key, keyLen), UTF8ToString(text, len));
      return true;
    } catch {
      return false;
    }
  },
});
