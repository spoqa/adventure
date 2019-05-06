pub use std::task::{Context, Poll, Waker};

pub use self::noop_waker::{noop_waker, noop_waker_ref};

/// Utilities for creating zero-cost wakers that don't do anything.
mod noop_waker {

    use std::cell::UnsafeCell;
    use std::ptr::null;
    use std::task::{RawWaker, RawWakerVTable, Waker};
    unsafe fn noop_clone(_data: *const ()) -> RawWaker {
        noop_raw_waker()
    }

    unsafe fn noop(_data: *const ()) {}

    const NOOP_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop, noop, noop);

    fn noop_raw_waker() -> RawWaker {
        RawWaker::new(null(), &NOOP_WAKER_VTABLE)
    }

    /// Create a new [`Waker`](futures_core::task::Waker) which does
    /// nothing when `wake()` is called on it. The [`Waker`] can be converted
    /// into a [`Waker`] which will behave the same way.
    #[inline]
    pub fn noop_waker() -> Waker {
        unsafe { Waker::from_raw(noop_raw_waker()) }
    }

    /// Get a thread local reference to a
    /// [`Waker`](futures_core::task::Waker) referencing a singleton
    /// instance of a [`Waker`] which panics when woken.
    #[inline]
    pub fn noop_waker_ref() -> &'static Waker {
        thread_local! {
            static NOOP_WAKER_INSTANCE: UnsafeCell<Waker> =
                UnsafeCell::new(noop_waker());
        }
        NOOP_WAKER_INSTANCE.with(|l| unsafe { &*l.get() })
    }
}
