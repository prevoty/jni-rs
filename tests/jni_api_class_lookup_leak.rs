#![cfg(feature = "invocation")]
extern crate jni;
extern crate error_chain;
#[macro_use]
extern crate lazy_static;


use jni::{AttachGuard, InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use jni::errors::Result;
use jni::objects::{JObject, JValue};
use jni::sys::jint;

mod util;


lazy_static! {
  static ref JVM: JavaVM = JavaVM::new(
    InitArgsBuilder::new()
        .version(JNIVersion::V8)
        // Since we don't allocate anything on Java heap (we leak the native JVM heap),
        // we can't limit the memory size with JVM options to get an OoME.
        .build()
        .unwrap()
        ).unwrap();
}

#[test]
fn class_lookup_leaks_local_references() {
    let env = JVM.attach_current_thread().unwrap();

    let x = JValue::from(-1 as jint);

    for i in 0..10_000_000 {
        // The following call leaks a local reference to Class<Math> each time it is called
        let abs_x = env.call_static_method("java/lang/Math", "abs", "(I)I", &[x])
            .unwrap()
            .i()
            .unwrap();

        assert_eq!(1, abs_x);

        if i % (100 * 1024) == 0 {
            println!("Leaked {} local refs", i);
        }
    }
}

#[test]
fn class_lookup_leaking_local_references_workaround() {
    let env = JVM.attach_current_thread().unwrap();

    let x = JValue::from(-1 as jint);
    env.with_local_frame(32 as i32, || {
        let abs_x = env.call_static_method("java/lang/Math", "abs", "(I)I", &[x])
            .unwrap()
            .i()
            .unwrap();

        assert_eq!(1, abs_x);

        Ok(JObject::null())
    }).unwrap();
}

