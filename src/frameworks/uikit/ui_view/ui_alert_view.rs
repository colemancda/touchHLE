/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIAlertView`.
//!
//! Local patch: alerts are shown as a native (SDL) dialog with the alert's
//! buttons, on the run loop pass after `show`, and the delegate is told which
//! button was clicked, as on iOS.

use crate::frameworks::foundation::{ns_string, NSInteger};
use crate::objc::{id, msg, msg_super, nil, objc_classes, release, retain, ClassExports, SEL};
use crate::Environment;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Default, Clone)]
struct AlertState {
    title: String,
    message: String,
    /// non-retaining, as on iOS
    delegate: id,
    buttons: Vec<String>,
    cancel_index: NSInteger,
    visible: bool,
}

thread_local! {
    static ALERTS: RefCell<HashMap<u32, AlertState>> = RefCell::new(HashMap::new());
}

fn with<R>(this: id, f: impl FnOnce(&mut AlertState) -> R) -> R {
    ALERTS.with(|a| {
        f(a.borrow_mut()
            .entry(this.to_bits())
            .or_insert_with(|| AlertState {
                delegate: nil,
                cancel_index: -1,
                ..Default::default()
            }))
    })
}

fn string_or_empty(env: &mut Environment, s: id) -> String {
    if s == nil {
        String::new()
    } else {
        ns_string::to_rust_string(env, s).to_string()
    }
}

/// Sends `alertView:<selector>:` with a button index to the delegate, if it
/// implements it.
fn notify(env: &mut Environment, this: id, delegate: id, selector: &str, index: Option<NSInteger>) {
    if delegate == nil
        || !env
            .objc
            .object_has_method_named(&env.mem, delegate, selector)
    {
        return;
    }
    let sel: SEL = env.objc.lookup_selector(selector).unwrap();
    match index {
        Some(i) => crate::objc::msg_send(env, (delegate, sel, this, i)),
        None => crate::objc::msg_send(env, (delegate, sel, this)),
    }
}

fn dismiss(env: &mut Environment, this: id, index: NSInteger) {
    let delegate = with(this, |a| {
        a.visible = false;
        a.delegate
    });
    notify(
        env,
        this,
        delegate,
        "alertView:willDismissWithButtonIndex:",
        Some(index),
    );
    notify(
        env,
        this,
        delegate,
        "alertView:didDismissWithButtonIndex:",
        Some(index),
    );
    // balances the retain in -show
    release(env, this);
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIAlertView: UIView

- (id)initWithTitle:(id)title
            message:(id)message
           delegate:(id)delegate
  cancelButtonTitle:(id)cancel_button_title
  otherButtonTitles:(id)first_other, ...args {
    let this: id = msg_super![env; this init];
    let title = string_or_empty(env, title);
    let message = string_or_empty(env, message);
    let mut buttons = Vec::new();
    let mut cancel_index = -1;
    if cancel_button_title != nil {
        cancel_index = 0;
        buttons.push(string_or_empty(env, cancel_button_title));
    }
    if first_other != nil {
        buttons.push(string_or_empty(env, first_other));
        let mut varargs = args.start();
        loop {
            let next: id = varargs.next(env);
            if next == nil {
                break;
            }
            buttons.push(string_or_empty(env, next));
        }
    }
    log!("UIAlertView {:?}: title {:?}, message {:?}, buttons {:?}", this, title, message, buttons);
    with(this, |a| {
        a.title = title;
        a.message = message;
        a.delegate = delegate;
        a.buttons = buttons;
        a.cancel_index = cancel_index;
    });
    this
}

- (())setDelegate:(id)delegate {
    with(this, |a| a.delegate = delegate);
}
- (id)delegate {
    with(this, |a| a.delegate)
}
- (())setTitle:(id)title {
    let title = string_or_empty(env, title);
    with(this, |a| a.title = title);
}
- (id)title {
    let title = with(this, |a| a.title.clone());
    ns_string::from_rust_string(env, title)
}
- (())setMessage:(id)message {
    let message = string_or_empty(env, message);
    with(this, |a| a.message = message);
}
- (id)message {
    let message = with(this, |a| a.message.clone());
    ns_string::from_rust_string(env, message)
}

- (NSInteger)addButtonWithTitle:(id)title {
    let title = string_or_empty(env, title);
    with(this, |a| {
        a.buttons.push(title);
        (a.buttons.len() - 1) as NSInteger
    })
}
- (NSInteger)numberOfButtons {
    with(this, |a| a.buttons.len() as NSInteger)
}
- (id)buttonTitleAtIndex:(NSInteger)index {
    let title = with(this, |a| a.buttons.get(index as usize).cloned());
    match title {
        Some(t) => ns_string::from_rust_string(env, t),
        None => nil,
    }
}
- (NSInteger)cancelButtonIndex {
    with(this, |a| a.cancel_index)
}
- (())setCancelButtonIndex:(NSInteger)index {
    with(this, |a| a.cancel_index = index);
}
- (bool)isVisible {
    with(this, |a| a.visible)
}

- (())show {
    // Kept alive until dismissed (on iOS the alert window owns it).
    retain(env, this);
    with(this, |a| a.visible = true);
    let sel = env.objc.register_host_selector("_touchHLE_presentAlert".to_string(), &mut env.mem);
    () = msg![env; this performSelector:sel withObject:nil afterDelay:(0.0 as crate::frameworks::foundation::NSTimeInterval)];
}

- (())_touchHLE_presentAlert {
    let (visible, delegate, title, message, buttons, cancel_index) =
        with(this, |a| (a.visible, a.delegate, a.title.clone(), a.message.clone(), a.buttons.clone(), a.cancel_index));
    if !visible {
        return; // dismissed by the app before it was shown
    }
    notify(env, this, delegate, "willPresentAlertView:", None);
    notify(env, this, delegate, "didPresentAlertView:", None);
    let clicked = crate::window::show_alert_messagebox(env.window.as_deref(), &title, &message, &buttons);
    // Closing the dialog counts as the cancel button (or the first button).
    let index = clicked.map(|i| i as NSInteger).unwrap_or(if cancel_index >= 0 { cancel_index } else { 0 });
    log!("UIAlertView {:?}: button {} ({:?}) clicked", this, index, buttons.get(index as usize));
    notify(env, this, delegate, "alertView:clickedButtonAtIndex:", Some(index));
    dismiss(env, this, index);
}

- (())dismissWithClickedButtonIndex:(NSInteger)index animated:(bool)_animated {
    if with(this, |a| a.visible) {
        dismiss(env, this, index);
    }
}

@end

};
