use std::convert::From;
use std::sync::Arc;

use JavaVM;
use JNIEnv;
use errors::Result;
use objects::JObject;
use sys;


/// A global JVM reference. These are "pinned" by the garbage collector and are
/// guaranteed to not get collected until released. Thus, this is allowed to
/// outlive the `JNIEnv` that it came from and can be used in other threads.
#[derive(Clone)]
pub struct GlobalRef {
    inner: Arc<GlobalRefGuard>
}


struct GlobalRefGuard {
    obj: JObject<'static>,
    vm: JavaVM,
}


unsafe impl Send for GlobalRef {}


impl<'a> From<&'a GlobalRef> for JObject<'a> {
    fn from(other: &'a GlobalRef) -> JObject<'a> {
        other.as_obj()
    }
}


impl GlobalRef {
    /// Creates a new global reference. This assumes that `NewGlobalRef`
    /// has already been called.
    pub(crate) unsafe fn new(vm: JavaVM, obj: sys::jobject) -> Self {
        GlobalRef {
            inner: Arc::new(GlobalRefGuard::new(vm, obj)),
        }
    }

    /// Get the object from the global ref
    ///
    /// This borrows the ref and prevents it from being dropped as long as the
    /// JObject sticks around.
    pub fn as_obj<'a>(&'a self) -> JObject<'a> {
        self.inner.as_obj()
    }
}


impl GlobalRefGuard {
    /// Creates a new global reference. This assumes that `NewGlobalRef`
    /// has already been called.
    unsafe fn new(vm: JavaVM, obj: sys::jobject) -> Self {
        GlobalRefGuard {
            obj: JObject::from(obj),
            vm,
        }
    }

    /// Get the object from the global ref
    ///
    /// This borrows the ref and prevents it from being dropped as long as the
    /// JObject sticks around.
    pub fn as_obj<'a>(&'a self) -> JObject<'a> {
        self.obj
    }
}

impl Drop for GlobalRefGuard {
    fn drop(&mut self) {
        fn drop_impl(env: &JNIEnv, global_ref: JObject) -> Result<()> {
            let internal = env.get_native_interface();
            unsafe {
                jni_unchecked!(internal, DeleteGlobalRef, global_ref.into_inner());
                check_exception!(internal);
            }
            Ok(())
        }

        let res = match self.vm.get_env() {
            Ok(env) => drop_impl(&env, self.as_obj()),
            Err(_) => self.vm
                .attach_current_thread()
                .and_then(|env| drop_impl(&env, self.as_obj())),
        };

        if let Err(err) = res  {
            debug!("error dropping global ref: {:#?}", err);
        }
    }
}
