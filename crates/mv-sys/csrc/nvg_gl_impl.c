/* NanoVG GL backend implementation translation unit: GL3 natively, GLES3 on Emscripten. */
#if defined(__EMSCRIPTEN__)
#include <GLES3/gl3.h>
#elif defined(__APPLE__)
#include <OpenGL/gl3.h>
#else
#include <GL/gl.h>
#endif
#include "nanovg.h"
#if defined(__EMSCRIPTEN__)
#define NANOVG_GLES3_IMPLEMENTATION
#else
#define NANOVG_GL3_IMPLEMENTATION
#endif
#include "nanovg_gl.h"
// math-viz-rust: the framebuffer helpers' bodies live behind
// NANOVG_GL_IMPLEMENTATION; without it the declared nvglu* functions never
// link (found when the icon and harness paths reached Framebuffer).
#define NANOVG_GL_IMPLEMENTATION
#include "nanovg_gl_utils.h"

// math-viz-rust: finishes the GL command stream (glFinish) for the harness's
// frame readback.
void nvgGLFinish(void)
{
	glFinish();
}

// math-viz-rust: reads a framebuffer's pixels into RGBA bytes, rows bottom to
// top as glReadPixels gives them, for the harness's PNG shots.
void nvgluReadPixels(NVGLUframebuffer* fb, int w, int h, unsigned char* out)
{
	glBindFramebuffer(GL_FRAMEBUFFER, fb->fbo);
	glReadPixels(0, 0, w, h, GL_RGBA, GL_UNSIGNED_BYTE, out);
}
