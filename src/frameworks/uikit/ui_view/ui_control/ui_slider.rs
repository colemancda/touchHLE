/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISlider`.
//!
//! Local patch: minimum, maximum and value are stored, and touching or dragging
//! sets the value from the touch position and sends ValueChanged. The slider is
//! still not drawn.

use super::{send_actions, UIControlEventValueChanged};
use crate::frameworks::core_graphics::{CGPoint, CGRect};
use crate::objc::{id, msg, msg_super, objc_classes, todo_objc_setter, ClassExports};
use crate::Environment;
use std::cell::RefCell;
use std::collections::HashMap;

/// (minimum, maximum, value) per slider, keyed by object address.
#[derive(Clone, Copy)]
struct Values {
    min: f32,
    max: f32,
    value: f32,
}

thread_local! {
    static SLIDERS: RefCell<HashMap<u32, Values>> = RefCell::new(HashMap::new());
}

fn get(this: id) -> Values {
    SLIDERS.with(|s| {
        s.borrow().get(&this.to_bits()).copied().unwrap_or(Values {
            min: 0.0,
            max: 1.0,
            value: 0.0,
        })
    })
}
fn set(this: id, v: Values) {
    SLIDERS.with(|s| s.borrow_mut().insert(this.to_bits(), v));
}

/// Sets the value from a touch's horizontal position and reports the change.
fn track(env: &mut Environment, this: id, touch: id, event: id) {
    let location: CGPoint = msg![env; touch locationInView:this];
    let bounds: CGRect = msg![env; this bounds];
    let mut v = get(this);
    let fraction = if bounds.size.width > 0.0 {
        ((location.x - bounds.origin.x) / bounds.size.width).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let new_value = v.min + fraction * (v.max - v.min);
    if new_value != v.value {
        v.value = new_value;
        set(this, v);
        send_actions(env, this, event, UIControlEventValueChanged);
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISlider: UIControl

- (id)initWithFrame:(CGRect)frame {
    log!("[(UISlider*){:?} initWithFrame:{:?}] TODO: UISlider won't be rendered.", this, frame);
    msg_super![env; this initWithFrame:frame]
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log!("[(UISlider*){:?} initWithCoder:{:?}] TODO: UISlider won't be rendered.", this, coder);
    msg_super![env; this initWithCoder:coder]
}

- (f32)minimumValue {
    get(this).min
}
- (())setMinimumValue:(f32)min {
    let mut v = get(this);
    v.min = min;
    v.value = v.value.max(min);
    set(this, v);
}
- (f32)maximumValue {
    get(this).max
}
- (())setMaximumValue:(f32)max {
    let mut v = get(this);
    v.max = max;
    v.value = v.value.min(max);
    set(this, v);
}
- (f32)value {
    get(this).value
}
- (())setValue:(f32)value {
    let mut v = get(this);
    v.value = value.clamp(v.min.min(v.max), v.max.max(v.min));
    set(this, v);
}
- (())setValue:(f32)value animated:(bool)_animated {
    msg![env; this setValue:value]
}

- (())setContinuous:(bool)_continuous {}
- (())setMinimumValueImage:(id)img { // UIImage *
    todo_objc_setter!(this, img);
}
- (())setMaximumValueImage:(id)img { // UIImage *
    todo_objc_setter!(this, img);
}
- (())setThumbImage:(id)img forState:(super::UIControlState)_state { // UIImage *
    todo_objc_setter!(this, img);
}
- (())setMinimumTrackImage:(id)img forState:(super::UIControlState)_state { // UIImage *
    todo_objc_setter!(this, img);
}
- (())setMaximumTrackImage:(id)img forState:(super::UIControlState)_state { // UIImage *
    todo_objc_setter!(this, img);
}

- (bool)beginTrackingWithTouch:(id)touch withEvent:(id)event {
    track(env, this, touch, event);
    true
}
- (bool)continueTrackingWithTouch:(id)touch withEvent:(id)event {
    track(env, this, touch, event);
    true
}

@end

};
