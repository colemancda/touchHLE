/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGContext.h`

use super::cg_affine_transform::{CGAffineTransform, CGAffineTransformIdentity};
use super::cg_bitmap_context::{
    CGBitmapContextDrawer, CGBitmapContextGetHeight, CGBitmapContextGetWidth,
};
use super::cg_color::CGColorRef;
use super::cg_color_space::{
    kCGColorSpaceModelMonochrome, kCGColorSpaceModelRGB, CGColorSpaceGetModel, CGColorSpaceRef,
};
use super::cg_font::{CGFontHostObject, CGFontRef, CGFontRelease, CGFontRetain, CGGlyph};
use super::cg_geometry::CGPointZero;
use super::cg_image::CGImageRef;
use super::{cg_bitmap_context, cg_color, CGFloat, CGPoint, CGRect, CGSize};
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::uikit;
use crate::mem::{ConstPtr, GuestUSize};
use crate::objc::{objc_classes, ClassExports, HostObject};
use crate::Environment;

type CGInterpolationQuality = i32;

type CGTextDrawingMode = i32;
const kCGTextFill: CGTextDrawingMode = 0;
const kCGTextFillStroke: CGTextDrawingMode = 2;

pub type CGBlendMode = i32;
pub const kCGBlendModeNormal: CGBlendMode = 0;
pub const kCGBlendModeMultiply: CGBlendMode = 1;
pub const kCGBlendModeScreen: CGBlendMode = 2;
#[allow(unused)]
pub const kCGBlendModeOverlay: CGBlendMode = 3;
pub const kCGBlendModeDarken: CGBlendMode = 4;
pub const kCGBlendModeLighten: CGBlendMode = 5;
pub const kCGBlendModeCopy: CGBlendMode = 17;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// CGContext seems to be a CFType-based type, but in our implementation those
// are just Objective-C types, so we need a class for it, but its name is not
// visible anywhere.
@implementation _touchHLE_CGContext: NSObject

- (())dealloc {
    let host_obj = env.objc.borrow::<CGContextHostObject>(this);
    let CGContextSubclass::CGBitmapContext(bitmap_data) = host_obj.subclass;
    if bitmap_data.data_is_owned {
        env.mem.free(bitmap_data.data);
    }
    CGFontRelease(env, host_obj.font);

    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

// TODO: keep more states saved once they are implemented
type ContextState = (
    (CGFloat, CGFloat, CGFloat, CGFloat), // RGB fill color
    CGAffineTransform,                    // transform
    CGFontRef,                            // font
    CGFloat,                              // font size
    CGBlendMode,                          // blend mode
);

pub(super) struct CGContextHostObject {
    pub(super) subclass: CGContextSubclass,
    pub(super) rgb_fill_color: (CGFloat, CGFloat, CGFloat, CGFloat),
    pub(super) font: CGFontRef,
    pub(super) font_size: CGFloat,
    /// Current transform.
    pub(super) transform: CGAffineTransform,
    pub(super) blend_mode: CGBlendMode,
    /// Text transform.
    pub(super) text_transform: Option<CGAffineTransform>,
    pub(super) state_stack: Vec<ContextState>,
}
impl HostObject for CGContextHostObject {}

pub(super) enum CGContextSubclass {
    CGBitmapContext(cg_bitmap_context::CGBitmapContextData),
}

pub type CGContextRef = CFTypeRef;

pub fn CGContextRelease(env: &mut Environment, c: CGContextRef) {
    if !c.is_null() {
        CFRelease(env, c);
    }
}
pub fn CGContextRetain(env: &mut Environment, c: CGContextRef) -> CGContextRef {
    if !c.is_null() {
        CFRetain(env, c)
    } else {
        c
    }
}

fn CGContextSetBlendMode(env: &mut Environment, context: CGContextRef, blend_mode: CGBlendMode) {
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .blend_mode = blend_mode;
}

fn CGContextSetFillColorSpace(
    env: &mut Environment,
    _context: CGContextRef,
    space: CGColorSpaceRef,
) {
    let color_model = CGColorSpaceGetModel(env, space);
    assert!(color_model == kCGColorSpaceModelMonochrome || color_model == kCGColorSpaceModelRGB);
    // TODO
}

fn CGContextSetFillColorWithColor(env: &mut Environment, context: CGContextRef, color: CGColorRef) {
    // Local patch: a NULL colour leaves the fill colour unchanged, as on iOS;
    // anything else that isn't a CGColor (e.g. a UIColor) is asked for one.
    if color.is_null() {
        log!("CGContextSetFillColorWithColor: NULL colour, ignoring");
        return;
    }
    let is_cg_color = env
        .objc
        .get_host_object(color)
        .and_then(|h| h.as_any().downcast_ref::<cg_color::CGColorHostObject>())
        .is_some();
    let color = if is_cg_color {
        color
    } else {
        let class: crate::objc::Class = crate::msg![env; color class];
        log!(
            "CGContextSetFillColorWithColor: {:?} is a {}, not a CGColor",
            color,
            env.objc.get_class_name(class)
        );
        let cg: CGColorRef = crate::msg![env; color CGColor];
        if cg.is_null() {
            return;
        }
        cg
    };
    let (r, g, b, a) = cg_color::to_rgba(&env.objc, color);
    CGContextSetRGBFillColor(env, context, r, g, b, a)
}

pub fn CGContextSetRGBFillColor(
    env: &mut Environment,
    context: CGContextRef,
    red: CGFloat,
    green: CGFloat,
    blue: CGFloat,
    alpha: CGFloat,
) {
    let color = (red, green, blue, alpha);
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .rgb_fill_color = color;
}

fn CGContextSetGrayFillColor(
    env: &mut Environment,
    context: CGContextRef,
    gray: CGFloat,
    alpha: CGFloat,
) {
    let color = (gray, gray, gray, alpha);
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .rgb_fill_color = color;
}

fn CGContextSetGrayStrokeColor(
    _env: &mut Environment,
    context: CGContextRef,
    gray: CGFloat,
    alpha: CGFloat,
) {
    log!(
        "TODO: CGContextSetGrayStrokeColor({:?}, {}, {})",
        context,
        gray,
        alpha,
    );
}
fn CGContextSetRGBStrokeColor(
    _env: &mut Environment,
    context: CGContextRef,
    r: CGFloat,
    g: CGFloat,
    b: CGFloat,
    a: CGFloat,
) {
    log!(
        "TODO: CGContextSetRGBStrokeColor({:?}, {}, {}, {}, {})",
        context,
        r,
        g,
        b,
        a
    );
}

// Local patch: minimal paths. Points are collected per context and a filled
// path is drawn as its bounding box (so rounded boxes come out square).
// Strokes are accepted but not drawn.
thread_local! {
    static PATHS: std::cell::RefCell<std::collections::HashMap<u32, Vec<CGPoint>>> = Default::default();
}
fn path_add(context: CGContextRef, points: &[CGPoint]) {
    PATHS.with(|p| {
        p.borrow_mut()
            .entry(context.to_bits())
            .or_default()
            .extend_from_slice(points)
    });
}
fn CGContextBeginPath(_env: &mut Environment, context: CGContextRef) {
    PATHS.with(|p| p.borrow_mut().remove(&context.to_bits()));
}
fn CGContextMoveToPoint(_env: &mut Environment, context: CGContextRef, x: CGFloat, y: CGFloat) {
    path_add(context, &[CGPoint { x, y }]);
}
fn CGContextAddArcToPoint(
    _env: &mut Environment,
    context: CGContextRef,
    x1: CGFloat,
    y1: CGFloat,
    x2: CGFloat,
    y2: CGFloat,
    _radius: CGFloat,
) {
    path_add(
        context,
        &[CGPoint { x: x1, y: y1 }, CGPoint { x: x2, y: y2 }],
    );
}
fn CGContextAddRect(_env: &mut Environment, context: CGContextRef, rect: CGRect) {
    let CGRect { origin: o, size: s } = rect;
    path_add(
        context,
        &[
            o,
            CGPoint {
                x: o.x + s.width,
                y: o.y + s.height,
            },
        ],
    );
}
fn CGContextClosePath(_env: &mut Environment, _context: CGContextRef) {}
fn CGContextFillPath(env: &mut Environment, context: CGContextRef) {
    let points = PATHS
        .with(|p| p.borrow_mut().remove(&context.to_bits()))
        .unwrap_or_default();
    if points.is_empty() {
        return;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in &points {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    let rect = CGRect {
        origin: CGPoint { x: x0, y: y0 },
        size: CGSize {
            width: x1 - x0,
            height: y1 - y0,
        },
    };
    CGContextFillRect(env, context, rect);
}
fn CGContextSetLineWidth(_env: &mut Environment, _context: CGContextRef, _width: CGFloat) {}
fn CGContextSetStrokeColor(
    _env: &mut Environment,
    _context: CGContextRef,
    _components: ConstPtr<CGFloat>,
) {
}
fn CGContextSetStrokeColorSpace(
    _env: &mut Environment,
    _context: CGContextRef,
    _space: CGColorSpaceRef,
) {
}
fn CGContextStrokeLineSegments(
    _env: &mut Environment,
    _context: CGContextRef,
    _points: ConstPtr<CGPoint>,
    _count: GuestUSize,
) {
    log!("TODO: CGContextStrokeLineSegments (lines aren't drawn)");
}
/// Components in the current fill colour space, which is always RGB(A) here.
fn CGContextSetFillColor(
    env: &mut Environment,
    context: CGContextRef,
    components: ConstPtr<CGFloat>,
) {
    let c: Vec<CGFloat> = (0..4).map(|i| env.mem.read(components + i)).collect();
    CGContextSetRGBFillColor(env, context, c[0], c[1], c[2], c[3]);
}

fn CGContextSetShadowWithColor(
    _env: &mut Environment,
    context: CGContextRef,
    offset: CGSize,
    blur: CGFloat,
    color: CGColorRef,
) {
    log!(
        "TODO: CGContextSetShadowWithColor({:?}, {}, {}, {:?})",
        context,
        offset,
        blur,
        color
    );
}

pub fn CGContextFillRect(env: &mut Environment, context: CGContextRef, rect: CGRect) {
    cg_bitmap_context::fill_rect(env, context, rect, /* clear: */ false);
}

pub fn CGContextClearRect(env: &mut Environment, context: CGContextRef, rect: CGRect) {
    cg_bitmap_context::fill_rect(env, context, rect, /* clear: */ true);
}

fn CGContextClipToRect(env: &mut Environment, context: CGContextRef, rect: CGRect) {
    if rect.origin == CGPointZero
        && rect.size.height == CGBitmapContextGetHeight(env, context) as f32
        && rect.size.width == CGBitmapContextGetWidth(env, context) as f32
    {
        assert!(env
            .objc
            .borrow_mut::<CGContextHostObject>(context)
            .transform
            .is_identity());
        // All good, clipping is not needed!
        return;
    }
    todo!();
}

pub fn CGContextConcatCTM(
    env: &mut Environment,
    context: CGContextRef,
    transform: CGAffineTransform,
) {
    log_dbg!("CGContextConcatCTM({:?})", transform);
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj.transform = transform.concat(host_obj.transform);
}
pub fn CGContextGetCTM(env: &mut Environment, context: CGContextRef) -> CGAffineTransform {
    let res = env.objc.borrow::<CGContextHostObject>(context).transform;
    log_dbg!("CGContextGetCTM() => {:?}", res);
    res
}
pub fn CGContextRotateCTM(env: &mut Environment, context: CGContextRef, angle: CGFloat) {
    log_dbg!("CGContextRotateCTM({:?})", angle);
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj.transform = host_obj.transform.rotate(angle);
}
pub fn CGContextScaleCTM(env: &mut Environment, context: CGContextRef, x: CGFloat, y: CGFloat) {
    log_dbg!("CGContextScaleCTM({:?})", (x, y));
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj.transform = host_obj.transform.scale(x, y);
}
pub fn CGContextTranslateCTM(
    env: &mut Environment,
    context: CGContextRef,
    tx: CGFloat,
    ty: CGFloat,
) {
    log_dbg!("CGContextTranslateCTM({:?})", (tx, ty));
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj.transform = host_obj.transform.translate(tx, ty);
}

pub fn CGContextDrawImage(
    env: &mut Environment,
    context: CGContextRef,
    rect: CGRect,
    image: CGImageRef,
) {
    cg_bitmap_context::draw_image(env, context, rect, image);
}

fn CGContextSaveGState(env: &mut Environment, context: CGContextRef) {
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    host_obj.state_stack.push((
        host_obj.rgb_fill_color,
        host_obj.transform,
        host_obj.font,
        host_obj.font_size,
        host_obj.blend_mode,
    ));
    CGFontRetain(env, env.objc.borrow::<CGContextHostObject>(context).font);
}

fn CGContextRestoreGState(env: &mut Environment, context: CGContextRef) {
    // We need to release _old_ font, there are 2 cases:
    // - font hasn't been set between save/restore -> this release corresponds
    // the font retain from save
    // - font has been set between save/restore -> we need to release old font
    // retained on the set
    CGFontRelease(env, env.objc.borrow::<CGContextHostObject>(context).font);
    let host_obj = env.objc.borrow_mut::<CGContextHostObject>(context);
    let state = host_obj.state_stack.pop().unwrap();
    host_obj.rgb_fill_color = state.0;
    host_obj.transform = state.1;
    host_obj.font = state.2;
    host_obj.font_size = state.3;
    host_obj.blend_mode = state.4;
}

fn CGContextSetInterpolationQuality(
    _env: &mut Environment,
    context: CGContextRef,
    quality: CGInterpolationQuality,
) {
    log!(
        "TODO: CGContextSetInterpolationQuality({:?}, {:?})",
        context,
        quality
    );
}
fn CGContextSetAllowsAntialiasing(_env: &mut Environment, context: CGContextRef, allow: bool) {
    log!(
        "TODO: CGContextSetAllowsAntialiasing({:?}, {})",
        context,
        allow
    );
}

fn CGContextSetShouldSmoothFonts(_env: &mut Environment, context: CGContextRef, should: bool) {
    log!(
        "TODO: CGContextSetShouldSmoothFonts({:?}, {})",
        context,
        should
    );
}

fn CGContextSetFont(env: &mut Environment, context: CGContextRef, font: CGFontRef) {
    CGFontRetain(env, font);
    let old_font = env.objc.borrow_mut::<CGContextHostObject>(context).font;
    CGFontRelease(env, old_font);
    env.objc.borrow_mut::<CGContextHostObject>(context).font = font;
}

fn CGContextSetFontSize(env: &mut Environment, context: CGContextRef, size: CGFloat) {
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .font_size = size;
}

fn CGContextSetTextDrawingMode(
    _env: &mut Environment,
    _context: CGContextRef,
    mode: CGTextDrawingMode,
) {
    // Local patch: invisible (3) and clip-only (7) text is not drawn; every other
    // mode (stroke, *Clip) is drawn as filled text, as fill was before.
    let invisible = mode == 3 || mode == 7;
    if !(mode == kCGTextFill || mode == kCGTextFillStroke || invisible) {
        log!(
            "TODO: CGContextSetTextDrawingMode({}): drawing as kCGTextFill",
            mode
        );
    }
    INVISIBLE_TEXT.with(|s| {
        if invisible {
            s.borrow_mut().insert(_context.to_bits());
        } else {
            s.borrow_mut().remove(&_context.to_bits());
        }
    });
}

thread_local! {
    /// Local patch: contexts whose text drawing mode makes text invisible.
    static INVISIBLE_TEXT: std::cell::RefCell<std::collections::HashSet<u32>> = Default::default();
    /// Local patch: current text position per context.
    static TEXT_POSITION: std::cell::RefCell<std::collections::HashMap<u32, (CGFloat, CGFloat)>> = Default::default();
}

fn CGContextSetTextPosition(_env: &mut Environment, context: CGContextRef, x: CGFloat, y: CGFloat) {
    TEXT_POSITION.with(|p| p.borrow_mut().insert(context.to_bits(), (x, y)));
}

fn CGContextGetTextPosition(_env: &mut Environment, context: CGContextRef) -> CGPoint {
    let (x, y) = TEXT_POSITION.with(|p| {
        p.borrow()
            .get(&context.to_bits())
            .copied()
            .unwrap_or((0.0, 0.0))
    });
    CGPoint { x, y }
}

fn CGContextShowGlyphs(
    env: &mut Environment,
    context: CGContextRef,
    glyphs: ConstPtr<CGGlyph>,
    count: GuestUSize,
) {
    let (x, y) = TEXT_POSITION.with(|p| {
        p.borrow()
            .get(&context.to_bits())
            .copied()
            .unwrap_or((0.0, 0.0))
    });
    CGContextShowGlyphsAtPoint(env, context, x, y, glyphs, count)
}

fn CGContextSetTextMatrix(
    env: &mut Environment,
    context: CGContextRef,
    transform: CGAffineTransform,
) {
    log_dbg!("CGContextSetTextMatrix({:?})", transform);
    env.objc
        .borrow_mut::<CGContextHostObject>(context)
        .text_transform = Some(transform);
}

fn CGContextShowGlyphsAtPoint(
    env: &mut Environment,
    context: CGContextRef,
    x: CGFloat,
    y: CGFloat,
    glyphs: ConstPtr<CGGlyph>,
    count: GuestUSize,
) {
    // Local patch: advance the text position past these glyphs (apps measure
    // text by drawing it invisibly and reading the position back).
    {
        let host = env.objc.borrow::<CGContextHostObject>(context);
        let (font_ref, size) = (host.font, host.font_size);
        let font = &env.objc.borrow::<CGFontHostObject>(font_ref).font;
        let scale = size / font.units_per_em() as CGFloat;
        let mut advance: CGFloat = 0.0;
        for i in 0..count {
            let glyph_id: CGGlyph = env.mem.read(glyphs + i);
            advance += font.glyph_hor_advance(glyph_id).unwrap_or(0) as CGFloat * scale;
        }
        TEXT_POSITION.with(|p| p.borrow_mut().insert(context.to_bits(), (x + advance, y)));
    }
    if INVISIBLE_TEXT.with(|s| s.borrow().contains(&context.to_bits())) {
        return;
    }
    let mut glyph_ids = Vec::new();
    for i in 0..count {
        let glyph_id = env.mem.read(glyphs + i);
        glyph_ids.push(rusttype::GlyphId(glyph_id));
    }

    let font = env.objc.borrow::<CGContextHostObject>(context).font;
    let font_size = env.objc.borrow::<CGContextHostObject>(context).font_size;
    let text_transform = env
        .objc
        .borrow::<CGContextHostObject>(context)
        .text_transform
        .unwrap_or(CGAffineTransformIdentity);

    let font = &env.objc.borrow::<CGFontHostObject>(font).font;

    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);
    let fill_color = drawer.rgb_fill_color();

    font.draw_glyphs(
        font_size,
        glyph_ids,
        (x, y),
        text_transform,
        |raster_glyph| {
            uikit::ui_font::draw_font_glyph(
                &mut drawer,
                raster_glyph,
                fill_color,
                /* clip_x: */ None,
                /* clip_y: */ None,
            )
        },
    );
}

fn CGContextShowGlyphsAtPositions(
    env: &mut Environment,
    context: CGContextRef,
    glyphs: ConstPtr<CGGlyph>,
    positions: ConstPtr<CGPoint>,
    count: GuestUSize,
) {
    let text_transform = env
        .objc
        .borrow::<CGContextHostObject>(context)
        .text_transform
        .unwrap_or(CGAffineTransformIdentity);
    assert!(text_transform.tx == 0.0 && text_transform.ty == 0.0); // TODO

    for i in 0..count {
        let glyph_ptr = glyphs + i;
        let point = env.mem.read(positions + i);
        let transformed_point = text_transform.apply_to_point(point);
        CGContextShowGlyphsAtPoint(
            env,
            context,
            transformed_point.x,
            transformed_point.y,
            glyph_ptr,
            1,
        );
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGContextRetain(_)),
    export_c_func!(CGContextRelease(_)),
    export_c_func!(CGContextSetBlendMode(_, _)),
    export_c_func!(CGContextSetFillColorSpace(_, _)),
    export_c_func!(CGContextSetFillColorWithColor(_, _)),
    export_c_func!(CGContextSetRGBFillColor(_, _, _, _, _)),
    export_c_func!(CGContextSetGrayFillColor(_, _, _)),
    export_c_func!(CGContextSetGrayStrokeColor(_, _, _)),
    export_c_func!(CGContextSetRGBStrokeColor(_, _, _, _, _)),
    export_c_func!(CGContextSetShadowWithColor(_, _, _, _)),
    export_c_func!(CGContextFillRect(_, _)),
    export_c_func!(CGContextBeginPath(_)),
    export_c_func!(CGContextMoveToPoint(_, _, _)),
    export_c_func!(CGContextAddArcToPoint(_, _, _, _, _, _)),
    export_c_func!(CGContextAddRect(_, _)),
    export_c_func!(CGContextClosePath(_)),
    export_c_func!(CGContextFillPath(_)),
    export_c_func!(CGContextSetLineWidth(_, _)),
    export_c_func!(CGContextSetStrokeColor(_, _)),
    export_c_func!(CGContextSetStrokeColorSpace(_, _)),
    export_c_func!(CGContextStrokeLineSegments(_, _, _)),
    export_c_func!(CGContextSetFillColor(_, _)),
    export_c_func!(CGContextClearRect(_, _)),
    export_c_func!(CGContextClipToRect(_, _)),
    export_c_func!(CGContextConcatCTM(_, _)),
    export_c_func!(CGContextGetCTM(_)),
    export_c_func!(CGContextRotateCTM(_, _)),
    export_c_func!(CGContextScaleCTM(_, _, _)),
    export_c_func!(CGContextTranslateCTM(_, _, _)),
    export_c_func!(CGContextDrawImage(_, _, _)),
    export_c_func!(CGContextSaveGState(_)),
    export_c_func!(CGContextRestoreGState(_)),
    export_c_func!(CGContextSetInterpolationQuality(_, _)),
    export_c_func!(CGContextSetAllowsAntialiasing(_, _)),
    export_c_func!(CGContextSetShouldSmoothFonts(_, _)),
    export_c_func!(CGContextSetFont(_, _)),
    export_c_func!(CGContextSetFontSize(_, _)),
    export_c_func!(CGContextSetTextDrawingMode(_, _)),
    export_c_func!(CGContextSetTextMatrix(_, _)),
    export_c_func!(CGContextShowGlyphsAtPoint(_, _, _, _, _)),
    export_c_func!(CGContextSetTextPosition(_, _, _)),
    export_c_func!(CGContextGetTextPosition(_)),
    export_c_func!(CGContextShowGlyphs(_, _, _)),
    export_c_func!(CGContextShowGlyphsAtPositions(_, _, _, _)),
];
