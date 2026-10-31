/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFPreferences`.
//!
//! According to Apple's docs, it's not toll-free bridged to `NSUserDefaults`,
//! but we are still implementing one atop of another.

use super::cf_string::CFStringRef;
use super::CFTypeRef;
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::foundation::ns_string;
use crate::objc::{id, msg, msg_class};
use crate::Environment;

type CFPropertyListRef = CFTypeRef;

fn CFPreferencesCopyAppValue(
    env: &mut Environment,
    key: CFStringRef,
    app_id: CFStringRef,
) -> CFPropertyListRef {
    let current_app = ns_string::get_static_str(env, kCFPreferencesCurrentApplication);
    // TODO: handle other ids
    assert!(msg![env; app_id isEqualToString:current_app]);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    let value: id = msg![env; user_defaults objectForKey:key];
    if std::env::var_os("TOUCHHLE_PREFS_LOG").is_some() {
        let c: crate::objc::Class = if value == crate::objc::nil {
            crate::objc::nil
        } else {
            msg![env; value class]
        };
        log!(
            "CFPreferencesCopyAppValue({:?}) -> {:?} ({})",
            ns_string::to_rust_string(env, key),
            value,
            if value == crate::objc::nil {
                "nil".to_string()
            } else {
                env.objc.get_class_name(c).to_string()
            }
        );
        let dict: id = msg![env; user_defaults dictionaryRepresentation];
        let keys: id = msg![env; dict allKeys];
        let n: crate::frameworks::foundation::NSUInteger = msg![env; keys count];
        for i in 0..n {
            let k: id = msg![env; keys objectAtIndex:i];
            let v: id = msg![env; dict objectForKey:k];
            let vc: crate::objc::Class = msg![env; v class];
            log!(
                "  defaults has {:?} = {:?} ({})",
                ns_string::to_rust_string(env, k),
                v,
                env.objc.get_class_name(vc)
            );
        }
    }
    msg![env; value copy]
}

fn CFPreferencesSetAppValue(
    env: &mut Environment,
    key: CFStringRef,
    value: CFPropertyListRef,
    app_id: CFStringRef,
) {
    let current_app = ns_string::get_static_str(env, kCFPreferencesCurrentApplication);
    // TODO: handle other ids
    assert!(msg![env; app_id isEqualToString:current_app]);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    if std::env::var_os("TOUCHHLE_PREFS_LOG").is_some() {
        log!(
            "CFPreferencesSetAppValue({:?}, {:?})",
            ns_string::to_rust_string(env, key),
            value
        );
    }
    // Local patch: a NULL value removes the key, as in CoreFoundation.
    if value.is_null() {
        return msg![env; user_defaults removeObjectForKey:key];
    }
    msg![env; user_defaults setObject:value forKey:key]
}

fn CFPreferencesAppSynchronize(env: &mut Environment, app_id: CFStringRef) -> bool {
    let current_app = ns_string::get_static_str(env, kCFPreferencesCurrentApplication);
    // TODO: handle other ids
    assert!(msg![env; app_id isEqualToString:current_app]);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; user_defaults synchronize]
}

pub const kCFPreferencesCurrentApplication: &str = "kCFPreferencesCurrentApplication";

pub const CONSTANTS: ConstantExports = &[(
    "_kCFPreferencesCurrentApplication",
    HostConstant::NSString(kCFPreferencesCurrentApplication),
)];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFPreferencesCopyAppValue(_, _)),
    export_c_func!(CFPreferencesSetAppValue(_, _, _)),
    export_c_func!(CFPreferencesAppSynchronize(_)),
];
