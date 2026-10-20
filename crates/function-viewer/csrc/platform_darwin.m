// The macOS Objective-C behind `platform_darwin.rs`: wheel-event watching,
// display activity, reduced motion, colour-space tagging, the ICC profile,
// appearances and hand cursors.
//
// One `.m` file, no SDL headers: every function takes plain C types (the
// Cocoa window as `void *`, spaces as 0/1), and every string or buffer handed
// back is `malloc`'d for the Rust side to free. Compiled by `build.rs` on
// macOS only.
//
// Ported from `src/platform_darwin.odin` (324 lines): the AppKit call
// sequence is the spec, translated from `objc_send` to message syntax.

#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>

#include <stdlib.h>
#include <string.h>

// Whether any display is active. A background login has none (CoreGraphics
// lists no active display while NSScreen, which SDL reads, still lists the
// machine's screens).
bool mv_darwin_display_active(void) {
    uint32_t count = 0;
    return CGGetActiveDisplayList(0, NULL, &count) == 0 && count > 0;
}

// System Settings › Accessibility › Display › Reduce motion.
bool mv_darwin_reduced_motion(void) {
    @autoreleasepool {
        return [[NSWorkspace sharedWorkspace] accessibilityDisplayShouldReduceMotion];
    }
}

// Whether AppKit's current event is a wheel event without precise scrolling
// deltas (a mouse wheel's ticks, not a trackpad's). SDL's event watch runs as
// SDL takes each event from AppKit, where NSApp's current event is the scroll
// event itself.
bool mv_darwin_current_wheel_is_tick(void) {
    @autoreleasepool {
        NSEvent *event = [NSApp currentEvent];
        return event != nil && event.type == NSEventTypeScrollWheel &&
               !event.hasPreciseScrollingDeltas;
    }
}

// Tags the Cocoa window with the space (1 Display P3, else sRGB) between
// SDL's tag and the GL context, and says who tagged it: SDL tagged every
// Cocoa window sRGB as it made it, and an equal space needs no work.
void mv_darwin_tag_space(void *nswindow, int space, int *set_space, char **tagged_by_out) {
    @autoreleasepool {
        NSWindow *window = (__bridge NSWindow *)nswindow;
        NSColorSpace *want =
            space == 1 ? [NSColorSpace displayP3ColorSpace] : [NSColorSpace sRGBColorSpace];
        NSColorSpace *found = [window colorSpace];
        if (found != nil && [found isEqual:want]) {
            *set_space = space;
            *tagged_by_out = strdup("SDL");
            return;
        }
        NSString *name = found ? [found localizedName] : @"none";
        NSString *account =
            [NSString stringWithFormat:@"the app, over SDL's %@", name];
        [window setColorSpace:want];
        *set_space = space;
        *tagged_by_out = strdup([account UTF8String]);
    }
}

// The colour report: the window's space, the surface its frames go to, and
// the screen it is on. A login with no active display has no screen to show
// the frames on, so the line says so.
char *mv_darwin_color_report(void *nswindow, const char *tagged_by) {
    @autoreleasepool {
        NSWindow *window = (__bridge NSWindow *)nswindow;
        NSColorSpace *space = [window colorSpace];
        NSString *windowName = space ? [space localizedName] : @"none";
        NSString *surfaceWords = @"no GL surface";
        NSView *view = [window contentView];
        CALayer *layer = view ? [view layer] : nil;
        id contents = layer ? [layer contents] : nil;
        if (contents) {
            NSString *className = NSStringFromClass([contents class]);
            if ([contents respondsToSelector:@selector(colorSpace)]) {
                CGColorSpaceRef cg = (__bridge CGColorSpaceRef)[contents colorSpace];
                if (cg) {
                    CFStringRef raw = CGColorSpaceCopyName(cg);
                    NSString *cgName = raw
                        ? [NSString stringWithString:(__bridge NSString *)raw]
                        : @"an unnamed colour space";
                    if (raw) {
                        CFRelease(raw);
                    }
                    surfaceWords = [NSString
                        stringWithFormat:@"its %@ in %@", className, cgName];
                } else {
                    surfaceWords = [NSString stringWithFormat:@"its %@ in no colour "
                                                              @"space (nil)",
                                                              className];
                }
            } else {
                surfaceWords = [NSString stringWithFormat:@"its %@, which names no "
                                                          @"colour space",
                                                          className];
            }
        }
        NSString *screenWords = @"no display";
        if (mv_darwin_display_active()) {
            NSScreen *screen = [window screen];
            if (screen) {
                NSColorSpace *screenSpace = [screen colorSpace];
                screenWords = [NSString stringWithFormat:@"the screen's %@",
                                                         screenSpace
                                                             ? [screenSpace localizedName]
                                                             : @"none"];
            } else {
                screenWords = @"no screen";
            }
        }
        NSString *report = [NSString
            stringWithFormat:@"the window's %@ (tagged by %s), %@; %@",
                             windowName, tagged_by, surfaceWords, screenWords];
        return strdup([report UTF8String]);
    }
}

// The ICC profile of the space (1 Display P3, else sRGB) as macOS has it,
// copied out. Display P3's is the 536 bytes of
// `/System/Library/ColorSync/Profiles/Display P3.icc`.
bool mv_darwin_icc_profile(int space, unsigned char **out, unsigned long *len_out) {
    @autoreleasepool {
        NSColorSpace *cs =
            space == 1 ? [NSColorSpace displayP3ColorSpace] : [NSColorSpace sRGBColorSpace];
        NSData *data = [cs ICCProfileData];
        if (data == nil || [data length] == 0) {
            return false;
        }
        *len_out = [data length];
        *out = malloc([data length]);
        if (*out == NULL) {
            return false;
        }
        memcpy(*out, [data bytes], [data length]);
        return true;
    }
}

// Sets the app's appearance, which SDL observes and reports as
// `SDL_EVENT_SYSTEM_THEME_CHANGED`. Only the live driver calls it.
void mv_darwin_set_appearance(bool dark) {
    @autoreleasepool {
        NSAppearance *appearance = [NSAppearance
            appearanceNamed:dark ? NSAppearanceNameDarkAqua : NSAppearanceNameAqua];
        [[NSApplication sharedApplication] setAppearance:appearance];
    }
}

// The open (grab) or closed (grabbing) hand's base image as straight-alpha
// RGBA at its own size, with the hotspot. Nil for any other failure, and
// SDL's system cursor stands in. The denser alternates the Odin side kept are
// dropped: the OS cursor never appears in a shot.
unsigned char *mv_darwin_hand_cursor(bool grabbing, int *w, int *h, int *hotx,
                                     int *hoty) {
    @autoreleasepool {
        NSCursor *cursor =
            grabbing ? [NSCursor closedHandCursor] : [NSCursor openHandCursor];
        NSImage *image = [cursor image];
        NSPoint hot = [cursor hotSpot];
        NSSize size = [image size];
        NSData *tiff = [image TIFFRepresentation];
        NSArray *reps = [NSBitmapImageRep imageRepsWithData:tiff];
        if (reps == nil) {
            return NULL;
        }
        for (NSBitmapImageRep *rep in reps) {
            if ([rep bitsPerSample] != 8 || [rep samplesPerPixel] != 4 ||
                [rep isPlanar]) {
                continue;
            }
            NSUInteger format = [rep bitmapFormat];
            if (format & 1) { // alpha first: not the RGBA layout SDL takes
                continue;
            }
            if ((double)[rep pixelsWide] != size.width) {
                continue; // a denser alternate, not the base image
            }
            long width = [rep pixelsWide];
            long height = [rep pixelsHigh];
            long pitch = [rep bytesPerRow];
            unsigned char *bitmap = [rep bitmapData];
            if (bitmap == NULL) {
                return NULL;
            }
            unsigned char *out = malloc(4 * width * height);
            if (out == NULL) {
                return NULL;
            }
            for (long y = 0; y < height; y++) {
                memcpy(out + 4 * y * width, bitmap + y * pitch, 4 * width);
            }
            if (!(format & 2)) { // premultiplied: straighten the alpha
                for (long i = 0; i < width * height; i++) {
                    unsigned char a = out[4 * i + 3];
                    if (a == 0) {
                        out[4 * i] = out[4 * i + 1] = out[4 * i + 2] = 0;
                    } else if (a < 255) {
                        out[4 * i] = (out[4 * i] * 255 + a / 2) / a;
                        out[4 * i + 1] = (out[4 * i + 1] * 255 + a / 2) / a;
                        out[4 * i + 2] = (out[4 * i + 2] * 255 + a / 2) / a;
                    }
                }
            }
            *w = (int)width;
            *h = (int)height;
            *hotx = (int)hot.x;
            *hoty = (int)hot.y;
            return out;
        }
        return NULL;
    }
}
