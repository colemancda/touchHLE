/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UITabBarController`.
//!
//! Local patch, minimal: shows the selected tab's view full-screen. There is no
//! tab bar (`tabBar` is nil), so the app can't switch tabs by touch.

use crate::frameworks::core_graphics::CGRect;
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{ns_array, NSUInteger};
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes,
    release, retain, ClassExports, NSZonePtr,
};
use crate::Environment;

#[derive(Default)]
struct UITabBarControllerHostObject {
    superclass: super::UIViewControllerHostObject,
    /// something implementing UITabBarControllerDelegate, non-retaining
    delegate: id,
    /// Retained.
    view_controllers: Vec<id>,
    selected_index: NSUInteger,
    /// The view controller whose view is currently a subview of ours.
    shown: id,
}
impl_HostObject_with_superclass!(UITabBarControllerHostObject);

/// Puts the selected view controller's view inside our view, if ours is loaded.
fn show_selected(env: &mut Environment, this: id) {
    let host = env.objc.borrow::<UITabBarControllerHostObject>(this);
    let self_view = host.superclass.view;
    if self_view == nil {
        return;
    }
    let selected = host
        .view_controllers
        .get(host.selected_index as usize)
        .copied()
        .unwrap_or(nil);
    let shown = host.shown;
    if selected == shown {
        return;
    }
    if shown != nil {
        let old_view: id = msg![env; shown view];
        () = msg![env; shown viewWillDisappear:false];
        () = msg![env; old_view removeFromSuperview];
        () = msg![env; shown viewDidDisappear:false];
    }
    env.objc
        .borrow_mut::<UITabBarControllerHostObject>(this)
        .shown = selected;
    if selected == nil {
        return;
    }
    let vc_view: id = msg![env; selected view];
    let bounds: CGRect = msg![env; self_view bounds];
    () = msg![env; vc_view setFrame:bounds];
    () = msg![env; selected viewWillAppear:false];
    () = msg![env; self_view addSubview:vc_view];
    () = msg![env; selected viewDidAppear:false];
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITabBarController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UITabBarControllerHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let key = get_static_str(env, "UIViewControllers");
    let vcs: id = msg![env; coder decodeObjectForKey:key];
    if vcs != nil {
        () = msg![env; this setViewControllers:vcs];
    }
    this
}

- (())dealloc {
    let vcs = std::mem::take(&mut env.objc.borrow_mut::<UITabBarControllerHostObject>(this).view_controllers);
    for vc in vcs {
        release(env, vc);
    }
    msg_super![env; this dealloc]
}

- (())loadView {
    let screen: id = msg_class![env; UIScreen mainScreen];
    let frame: CGRect = msg![env; screen applicationFrame];
    let view: id = msg_class![env; UIView alloc];
    let view: id = msg![env; view initWithFrame:frame];
    () = msg![env; this setView:view];
    release(env, view);
    show_selected(env, this);
}

- (id)viewControllers {
    let vcs = env.objc.borrow::<UITabBarControllerHostObject>(this).view_controllers.clone();
    for vc in &vcs {
        retain(env, *vc);
    }
    let res = ns_array::from_vec(env, vcs);
    autorelease(env, res)
}
- (())setViewControllers:(id)controllers { // NSArray *
    let count: NSUInteger = if controllers == nil { 0 } else { msg![env; controllers count] };
    let mut new = Vec::new();
    for i in 0..count {
        let vc: id = msg![env; controllers objectAtIndex:i];
        retain(env, vc);
        new.push(vc);
    }
    let old = std::mem::replace(&mut env.objc.borrow_mut::<UITabBarControllerHostObject>(this).view_controllers, new);
    env.objc.borrow_mut::<UITabBarControllerHostObject>(this).selected_index = 0;
    show_selected(env, this);
    for vc in old {
        release(env, vc);
    }
}
- (())setViewControllers:(id)controllers animated:(bool)_animated {
    msg![env; this setViewControllers:controllers]
}

- (NSUInteger)selectedIndex {
    env.objc.borrow::<UITabBarControllerHostObject>(this).selected_index
}
- (())setSelectedIndex:(NSUInteger)index {
    env.objc.borrow_mut::<UITabBarControllerHostObject>(this).selected_index = index;
    show_selected(env, this);
}
- (id)selectedViewController {
    let host = env.objc.borrow::<UITabBarControllerHostObject>(this);
    host.view_controllers.get(host.selected_index as usize).copied().unwrap_or(nil)
}
- (())setSelectedViewController:(id)vc {
    let index = env.objc.borrow::<UITabBarControllerHostObject>(this).view_controllers.iter().position(|&v| v == vc);
    if let Some(index) = index {
        msg![env; this setSelectedIndex:(index as NSUInteger)]
    }
}

// weak/non-retaining
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<UITabBarControllerHostObject>(this).delegate = delegate;
}
- (id)delegate {
    env.objc.borrow::<UITabBarControllerHostObject>(this).delegate
}

- (id)tabBar {
    // TODO: no tab bar is drawn
    nil
}

@end

};
