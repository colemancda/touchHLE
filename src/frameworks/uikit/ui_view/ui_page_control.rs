/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPageControl`.
//!
//! Local patch: the current page and page count are stored, and a tap on the
//! right/left half moves one page forward/back and sends ValueChanged, as on
//! iOS. The dots are not drawn.

use super::ui_control::{send_actions, UIControlEventValueChanged};
use crate::frameworks::core_graphics::{CGPoint, CGRect};
use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, msg, objc_classes, ClassExports};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// (current page, number of pages) per control, keyed by object address.
    static PAGES: RefCell<HashMap<u32, (NSInteger, NSInteger)>> = RefCell::new(HashMap::new());
}

fn get(this: id) -> (NSInteger, NSInteger) {
    PAGES.with(|p| p.borrow().get(&this.to_bits()).copied().unwrap_or((0, 0)))
}
fn set(this: id, value: (NSInteger, NSInteger)) {
    PAGES.with(|p| p.borrow_mut().insert(this.to_bits(), value));
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIPageControl: UIControl

- (NSInteger)currentPage {
    get(this).0
}
- (())setCurrentPage:(NSInteger)current_page {
    let (_, count) = get(this);
    let page = if count > 0 { current_page.clamp(0, count - 1) } else { current_page.max(0) };
    set(this, (page, count));
}

- (NSInteger)numberOfPages {
    get(this).1
}
- (())setNumberOfPages:(NSInteger)number_of_pages {
    let (page, _) = get(this);
    let count = number_of_pages.max(0);
    set(this, (if count > 0 { page.min(count - 1) } else { 0 }, count));
}

- (())setHidesForSinglePage:(bool)_hides {}
- (())setDefersCurrentPageDisplay:(bool)_defers {}
- (())updateCurrentPageDisplay {}

- (())endTrackingWithTouch:(id)touch // UITouch*
                 withEvent:(id)event { // UIEvent*
    let location: CGPoint = msg![env; touch locationInView:this];
    let bounds: CGRect = msg![env; this bounds];
    let inside: bool = msg![env; this pointInside:location withEvent:event];
    if !inside {
        return;
    }
    let (page, count) = get(this);
    let mid = bounds.origin.x + bounds.size.width / 2.0;
    let new_page = if location.x >= mid { page + 1 } else { page - 1 };
    if count > 0 && new_page >= 0 && new_page < count {
        set(this, (new_page, count));
        send_actions(env, this, event, UIControlEventValueChanged);
    }
}

@end

};
