/* Sizes and field offsets of every struct that crosses the FFI, for the struct-size
   test (which compares them to Rust's). Extend alongside mv-sys's extern blocks: one
   MV_SIZE line per struct, one MV_OFF line per field the Rust side constructs or reads. */
#include <stddef.h>

#include "clay.h"
#include "nanovg.h"
#include "kb_text_shape.h"

#define MV_SIZE(Name, T) \
	size_t mv_sys_sizeof_##Name(void) { return sizeof(T); }
#define MV_OFF(Name, T, Field) \
	size_t mv_sys_offset_##Name##_##Field(void) { return offsetof(T, Field); }

/* NanoVG */
MV_SIZE(NVGcolor, NVGcolor)
MV_OFF(NVGcolor, NVGcolor, r)
MV_OFF(NVGcolor, NVGcolor, g)
MV_OFF(NVGcolor, NVGcolor, b)
MV_OFF(NVGcolor, NVGcolor, a)

MV_SIZE(NVGpaint, NVGpaint)
MV_OFF(NVGpaint, NVGpaint, xform)
MV_OFF(NVGpaint, NVGpaint, extent)
MV_OFF(NVGpaint, NVGpaint, radius)
MV_OFF(NVGpaint, NVGpaint, feather)
MV_OFF(NVGpaint, NVGpaint, innerColor)
MV_OFF(NVGpaint, NVGpaint, outerColor)
MV_OFF(NVGpaint, NVGpaint, image)

MV_SIZE(NVGscissor, NVGscissor)
MV_OFF(NVGscissor, NVGscissor, xform)
MV_OFF(NVGscissor, NVGscissor, extent)

MV_SIZE(NVGvertex, NVGvertex)
MV_OFF(NVGvertex, NVGvertex, x)
MV_OFF(NVGvertex, NVGvertex, y)
MV_OFF(NVGvertex, NVGvertex, u)
MV_OFF(NVGvertex, NVGvertex, v)

/* math-viz-rust: the shaped-run glyph. */
MV_SIZE(NVGplacedGlyph, NVGplacedGlyph)
MV_OFF(NVGplacedGlyph, NVGplacedGlyph, index)
MV_OFF(NVGplacedGlyph, NVGplacedGlyph, x)
MV_OFF(NVGplacedGlyph, NVGplacedGlyph, y)

MV_SIZE(NVGglyphPosition, NVGglyphPosition)
MV_OFF(NVGglyphPosition, NVGglyphPosition, str)
MV_OFF(NVGglyphPosition, NVGglyphPosition, x)
MV_OFF(NVGglyphPosition, NVGglyphPosition, minx)
MV_OFF(NVGglyphPosition, NVGglyphPosition, maxx)

/* Clay */
MV_SIZE(Clay_String, Clay_String)
MV_OFF(Clay_String, Clay_String, isStaticallyAllocated)
MV_OFF(Clay_String, Clay_String, length)
MV_OFF(Clay_String, Clay_String, chars)

MV_SIZE(Clay_Arena, Clay_Arena)
MV_OFF(Clay_Arena, Clay_Arena, nextAllocation)
MV_OFF(Clay_Arena, Clay_Arena, capacity)
MV_OFF(Clay_Arena, Clay_Arena, memory)

MV_SIZE(Clay_Dimensions, Clay_Dimensions)
MV_OFF(Clay_Dimensions, Clay_Dimensions, width)
MV_OFF(Clay_Dimensions, Clay_Dimensions, height)

MV_SIZE(Clay_Vector2, Clay_Vector2)
MV_OFF(Clay_Vector2, Clay_Vector2, x)
MV_OFF(Clay_Vector2, Clay_Vector2, y)

MV_SIZE(Clay_Color, Clay_Color)
MV_OFF(Clay_Color, Clay_Color, r)
MV_OFF(Clay_Color, Clay_Color, g)
MV_OFF(Clay_Color, Clay_Color, b)
MV_OFF(Clay_Color, Clay_Color, a)

MV_SIZE(Clay_BoundingBox, Clay_BoundingBox)
MV_OFF(Clay_BoundingBox, Clay_BoundingBox, x)
MV_OFF(Clay_BoundingBox, Clay_BoundingBox, y)
MV_OFF(Clay_BoundingBox, Clay_BoundingBox, width)
MV_OFF(Clay_BoundingBox, Clay_BoundingBox, height)

/* kb_text_shape: only the structs the shaper calls read by field. */
MV_SIZE(KbtsGlyph, struct kbts_glyph)
MV_OFF(KbtsGlyph, struct kbts_glyph, Id)
MV_OFF(KbtsGlyph, struct kbts_glyph, AdvanceX)
MV_OFF(KbtsGlyph, struct kbts_glyph, OffsetX)
MV_OFF(KbtsGlyph, struct kbts_glyph, OffsetY)
MV_OFF(KbtsGlyph, struct kbts_glyph, AttachGlyph)
MV_OFF(KbtsGlyph, struct kbts_glyph, UserIdOrCodepointIndex)

MV_SIZE(KbtsRun, kbts_run)
MV_OFF(KbtsRun, kbts_run, Font)
MV_OFF(KbtsRun, kbts_run, Glyphs)

MV_SIZE(KbtsShapeCodepoint, kbts_shape_codepoint)
MV_OFF(KbtsShapeCodepoint, kbts_shape_codepoint, UserId)

MV_SIZE(KbtsFontInfo2_1, kbts_font_info2_1)
MV_OFF(KbtsFontInfo2_1, kbts_font_info2_1, UnitsPerEm)
