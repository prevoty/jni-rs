use std::{convert::From, sync::Arc};

use log::{debug, warn};

use crate::{
    errors::Result,
    objects::{GlobalRef, JObject},
    sys, JNIEnv, JavaVM,
};

/// A *weak* global JVM reference. These are global in scope like
/// [`GlobalRef`], and may outlive the `JNIEnv` they came from, but are
/// *not* guaranteed to not get collected until released.
///
/// `WeakRef` can be cloned to use _the same_ weak reference in different
/// contexts. If you want to create yet another weak ref to the same java object, call
/// [`JNIEnv::new_weak_ref`] like this:
///
/// ```no_run
/// # use jni::{JNIEnv, objects::*};
/// # let some_weak_ref: WeakRef = unimplemented!();
/// # let env: JNIEnv = unimplemented!();
/// #
/// # let _ =
/// env.new_weak_ref(JObject::from(some_weak_ref.as_raw()))
/// # ;
/// ```
///
/// Underlying weak reference will be dropped, when the last instance
/// of `WeakRef` leaves its scope.
///
/// It is _recommended_ that a native thread that drops the weak reference is attached
/// to the Java thread (i.e., has an instance of `JNIEnv`). If the native thread is *not* attached,
/// the `WeakRef#drop` will print a warning and implicitly `attach` and `detach` it, which
/// significantly affects performance.

#[derive(Clone)]
pub struct WeakRef {
    inner: Arc<WeakRefGuard>,
}

struct WeakRefGuard {
    raw: sys::jweak,
    vm: JavaVM,
}

unsafe impl Send for WeakRef {}
unsafe impl Sync for WeakRef {}

impl WeakRef {
    /// Creates a new wrapper for a global reference.
    ///
    /// # Safety
    ///
    /// Expects a valid raw weak global reference that should be created with `NewWeakGlobalRef`
    /// JNI function.
    pub(crate) unsafe fn from_raw(vm: JavaVM, raw: sys::jweak) -> Self {
        WeakRef {
            inner: Arc::new(WeakRefGuard { raw, vm }),
        }
    }

    /// Returns the raw JNI weak reference.
    pub fn as_raw(&self) -> sys::jweak {
        self.inner.raw
    }

    /// Creates a new local reference to this object.
    ///
    /// This object may have already been garbage collected by the time this method is called. If
    /// so, this method returns `Ok(None)`. Otherwise, it returns `Ok(Some(r))` where `r` is the
    /// new local reference.
    ///
    /// If this method returns `Ok(Some(r))`, it is guaranteed that the object will not be garbage
    /// collected at least until `r` is deleted or becomes invalid.
    pub fn upgrade_local<'e>(&self, env: &JNIEnv<'e>) -> Result<Option<JObject<'e>>> {
        let r = env.new_local_ref(JObject::from(self.inner.raw))?;

        // Per JNI spec, `NewLocalRef` will return a null pointer if the object was GC'd.
        if r.into_inner().is_null() {
            Ok(None)
        } else {
            Ok(Some(r))
        }
    }

    /// Creates a new strong global reference to this object.
    ///
    /// This object may have already been garbage collected by the time this method is called. If
    /// so, this method returns `Ok(None)`. Otherwise, it returns `Ok(Some(r))` where `r` is the
    /// new strong global reference.
    ///
    /// If this method returns `Ok(Some(r))`, it is guaranteed that the object will not be garbage
    /// collected at least until `r` is dropped.
    pub fn upgrade_global(&self, env: &JNIEnv) -> Result<Option<GlobalRef>> {
        let r = env.new_global_ref(JObject::from(self.inner.raw))?;

        // Unlike `NewLocalRef`, the JNI spec does *not* guarantee that `NewGlobalRef` will return a
        // null pointer if the object was GC'd, so we'll have to check.
        if env.is_same_object(r.as_obj(), JObject::null())? {
            Ok(None)
        } else {
            Ok(Some(r))
        }
    }
}

impl Drop for WeakRefGuard {
    fn drop(&mut self) {
        fn drop_impl(env: &JNIEnv, raw: sys::jweak) -> Result<()> {
            let internal = env.get_native_interface();
            // This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
            jni_unchecked!(internal, DeleteWeakGlobalRef, raw);
            Ok(())
        }

        let res = match self.vm.get_env() {
            Ok(env) => drop_impl(&env, self.raw),
            Err(_) => {
                warn!("Dropping a WeakRef in a detached thread. Fix your code if this message appears frequently (see the WeakRef docs).");
                self.vm
                    .attach_current_thread()
                    .and_then(|env| drop_impl(&env, self.raw))
            }
        };

        if let Err(err) = res {
            debug!("error dropping weak ref: {:#?}", err);
        }
    }
}
