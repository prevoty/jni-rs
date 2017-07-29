use std::convert::{AsRef, From};

use errors::*;

use objects::JObject;

use sys::{jobject, JNIEnv};

/// A global JVM reference. These are "pinned" by the garbage collector and are
/// guaranteed to not get collected until released. Thus, this is allowed to
/// outlive the `JNIEnv` that it came from. Still can't cross thread boundaries
/// since it requires a pointer to the `JNIEnv` to do anything useful with it.
pub struct GlobalRef {
    obj: jobject,
    env: *mut JNIEnv,
}

impl<'a> AsRef<JObject<'a>> for GlobalRef {
    fn as_ref(&self) -> &JObject<'a> {
        unsafe { ::std::mem::transmute(&self.obj) }
    }
}

impl<'a> From<GlobalRef> for JObject<'a> {
    fn from(other: GlobalRef) -> JObject<'a> {
        other.obj.into()
    }
}

impl GlobalRef {
    /// Create a new global reference object. This assumes that
    /// `CreateGlobalRef` has already been called.
    pub unsafe fn new(env: *mut JNIEnv, obj: jobject) -> Self {
        GlobalRef {
            obj: obj,
            env: env,
        }
    }

    fn drop_ref(&mut self) -> Result<()> {
        unsafe {
            jni_unchecked!(self.env, DeleteGlobalRef, self.obj);
            check_exception!(self.env);
        }
        Ok(())
    }
}

impl Drop for GlobalRef {
    fn drop(&mut self) {
        let res = self.drop_ref();
        match res {
            Ok(()) => {}
            Err(e) => debug!("error dropping global ref: {:#?}", e),
        }
    }
}
