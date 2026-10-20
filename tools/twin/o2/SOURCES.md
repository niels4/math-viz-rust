# tools/twin/o2: the oracle's tree, copied

Byte-identical copies of the Odin app's sources the twin builds against, because the
oracle's checkouts are read-only and its stb/kb_text_shape bindings refuse to compile
without the built libraries beside them (which the twin builds here from these copies).

Origin: `/opt/dev/agent/src/math-viz-odin` at `3d23626a07c317652e020febf18678ce06cedbec`
(the plan's oracle hash; the kept worktree `claude-port` is at the same hash).

Copied 2026-10-18: `vendor/nanovg` (`nanovg.odin`, `gl/gl.odin`), `vendor/fontstash`
(`fontstash.odin`), `vendor/sdl3` (the binding's `.odin` files), `vendor/kb_text_shape`
(`*.odin`, `src/`), `vendor/stb` (bindings, `src/`), `vendor/gl`, `src/text` (`*.odin`),
`assets/fonts` (the 13 TTFs).

`o2.sha256` (beside this file) holds every copied file's sha256; `run` checks it on every
build and fails loudly on drift. The built libraries (`vendor/stb/lib/`,
`vendor/kb_text_shape/lib/`, both gitignored) are rebuilt when their sources are newer.
