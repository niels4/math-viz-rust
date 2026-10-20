package sdl3

when ODIN_OS == .Windows {
	@(export) foreign import lib { "SDL3.lib" }
} else when ODIN_ARCH == .wasm32 || ODIN_ARCH == .wasm64p32 {
	// math-viz-odin: emcc links SDL3 from Emscripten's port (-sUSE_SDL=3). On wasm Odin gives
	// the procedures of a library whose path ends in ".o" their plain names, which wasm-ld
	// resolves against the port; "system:SDL3" leaves them imports from a "system:SDL3" module.
	@(export) foreign import lib "SDL3.o"
} else {
	@(export) foreign import lib { "system:SDL3" }
}