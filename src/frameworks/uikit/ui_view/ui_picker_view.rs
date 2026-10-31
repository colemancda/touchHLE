/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPickerView`.
//!
//! Local patch: the data source, delegate and selected row of each component
//! are stored, so apps can set and read the selection. Still not drawn and
//! can't be scrolled by touch.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, nil, objc_classes, todo_objc_setter, ClassExports};
use std::cell::RefCell;
use std::collections::HashMap;

// TODO: rendering

#[derive(Default, Clone)]
struct PickerState {
    /// non-retaining
    data_source: id,
    /// non-retaining
    delegate: id,
    selected_rows: HashMap<NSInteger, NSInteger>,
}

thread_local! {
    static PICKERS: RefCell<HashMap<u32, PickerState>> = RefCell::new(HashMap::new());
}

fn with<R>(this: id, f: impl FnOnce(&mut PickerState) -> R) -> R {
    PICKERS.with(|p| {
        f(p.borrow_mut()
            .entry(this.to_bits())
            .or_insert_with(|| PickerState {
                data_source: nil,
                delegate: nil,
                selected_rows: HashMap::new(),
            }))
    })
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIPickerView: UIView

- (())setShowsSelectionIndicator:(bool)shows {
    todo_objc_setter!(this, shows);
}

- (())setDelegate:(id)delegate {
    with(this, |p| p.delegate = delegate);
}
- (id)delegate {
    with(this, |p| p.delegate)
}
- (())setDataSource:(id)data_source {
    with(this, |p| p.data_source = data_source);
}
- (id)dataSource {
    with(this, |p| p.data_source)
}

- (())selectRow:(NSInteger)row inComponent:(NSInteger)component animated:(bool)_animated {
    with(this, |p| p.selected_rows.insert(component, row));
}
- (NSInteger)selectedRowInComponent:(NSInteger)component {
    with(this, |p| p.selected_rows.get(&component).copied().unwrap_or(0))
}

- (())reloadAllComponents {}
- (())reloadComponent:(NSInteger)_component {}

@end

};
