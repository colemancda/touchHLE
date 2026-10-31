/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISegmentedControl`.

use crate::frameworks::core_graphics::{CGFloat, CGRect};
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::objc::{id, msg, msg_super, objc_classes, ClassExports};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// Local patch: selected segment per control (keyed by object address).
    static SELECTED: RefCell<HashMap<u32, NSInteger>> = RefCell::new(HashMap::new());
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISegmentedControl: UIControl

- (id)initWithFrame:(CGRect)frame {
    log!("[(UISegmentedControl*){:?} initWithFrame:{:?}] TODO: Implement UISegmentedControl. The control won't be rendered.", this, frame);
    msg_super![env; this initWithFrame:frame]
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log!("[(UISegmentedControl*){:?} initWithCoder:{:?}] TODO: Implement UISegmentedControl. The control won't be rendered.", this, coder);
    msg_super![env; this initWithCoder:coder]
}

// Local patch: accepted but not drawn. Only the selected index is remembered.
- (id)initWithItems:(id)items { // NSArray of NSString or UIImage
    log!("[(UISegmentedControl*){:?} initWithItems:{:?}] TODO: segments won't be rendered.", this, items);
    let this: id = msg![env; this initWithFrame:(CGRect::default())];
    SELECTED.with(|s| s.borrow_mut().insert(this.to_bits(), -1));
    this
}
- (NSInteger)selectedSegmentIndex {
    SELECTED.with(|s| s.borrow().get(&this.to_bits()).copied().unwrap_or(-1))
}
- (())setSelectedSegmentIndex:(NSInteger)index {
    SELECTED.with(|s| s.borrow_mut().insert(this.to_bits(), index));
}
- (())setSegmentedControlStyle:(NSInteger)_style {}
- (())setMomentary:(bool)_momentary {}
- (())setTintColor:(id)_color {}
- (())setTitle:(id)_title forSegmentAtIndex:(NSUInteger)_index {}
- (())setImage:(id)_image forSegmentAtIndex:(NSUInteger)_index {}
- (())setWidth:(CGFloat)_width forSegmentAtIndex:(NSUInteger)_index {}
- (())setEnabled:(bool)_enabled forSegmentAtIndex:(NSUInteger)_index {}

@end

// Undocumented class used by UISegmentedControl
@implementation UISegment: UIControl

- (id)initWithFrame:(CGRect)frame {
    log!("[(UISegment*){:?} initWithFrame:{:?}] Attempted to initialize undocumented class from outside of the NIB.", this, frame);
    unreachable!()
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log!("[(UISegment*){:?} initWithCoder:{:?}] TODO: Implement UISegment. The control won't be rendered.", this, coder);
    msg_super![env; this initWithCoder:coder]
}

// TODO: all of it

@end

};
