/// Utilities for working with `Request` and `Response` traits.
use crate::response::Response;

/// An extension trait for `Response`s that provides a variety of convenient
/// adapters.
pub trait ResponseExt {}

impl<T> ResponseExt for T where T: Response {}

/// Converts a `Response` to compatible with futures, both of futures 0.1
/// and `std::future`.
#[must_use = "futures do nothing unless polled"]
pub struct IntoFuture<T>(T);

impl<T> IntoFuture<T> {
    pub(crate) fn new(fut: T) -> Self {
        IntoFuture(fut)
    }
}

#[cfg(feature = "futures01")]
mod impl_futures01 {
    use futures::{Future as Future01, Poll as Poll01};

    use crate::response::Response;
    use crate::task::convert_std_to_01;

    use super::IntoFuture;

    impl<T> Future01 for IntoFuture<T>
    where
        T: Response + Unpin,
    {
        type Item = T::Ok;
        type Error = T::Error;

        fn poll(&mut self) -> Poll01<Self::Item, Self::Error> {
            internal::with_context(self, |inner, ctx| {
                convert_std_to_01(Response::poll(inner, ctx))
            })
        }
    }

    #[cfg(feature = "std-future")]
    mod internal {
        // Copied from futures 0.3.0-alpha.1
        // Should be replaced if `futures-api` has been stablized
        use std::mem;
        use std::pin::Pin;
        use std::sync::Arc;
        use std::task::{Context, RawWaker, RawWakerVTable};

        use futures::task as task01;
        use futures_util::task::{ArcWake, WakerRef};

        use super::IntoFuture;
        use crate::task::Waker;

        unsafe fn ptr_to_current<'a>(ptr: *const ()) -> &'a Current {
            &*(ptr as *const Current)
        }
        fn current_to_ptr(current: &Current) -> *const () {
            current as *const Current as *const ()
        }

        unsafe fn clone(ptr: *const ()) -> RawWaker {
            // Lazily create the `Arc` only when the waker is actually cloned.
            // FIXME: remove `transmute` when a `Waker` -> `RawWaker` conversion
            // function is landed in `core`.
            mem::transmute::<Waker, RawWaker>(Arc::new(ptr_to_current(ptr).clone()).into_waker())
        }
        unsafe fn drop(_: *const ()) {}
        unsafe fn wake(ptr: *const ()) {
            ptr_to_current(ptr).0.notify()
        }
        unsafe fn wake_by_ref(ptr: *const ()) {
            ptr_to_current(ptr).0.notify()
        }

        const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

        #[derive(Clone)]
        struct Current(task01::Task);

        impl Current {
            fn new() -> Current {
                Current(task01::current())
            }

            fn as_waker(&self) -> WakerRef<'_> {
                let ptr = current_to_ptr(self);
                unsafe { WakerRef::new(Waker::from_raw(RawWaker::new(ptr, &VTABLE))) }
            }
        }

        impl ArcWake for Current {
            fn wake_by_ref(arc_self: &Arc<Self>) {
                arc_self.0.notify();
            }
        }

        pub(super) fn with_context<T, R, F>(fut: &mut IntoFuture<T>, f: F) -> R
        where
            T: Unpin,
            F: FnOnce(Pin<&mut T>, &mut Context<'_>) -> R,
        {
            let current = Current::new();
            let waker = current.as_waker();
            let mut ctx = Context::from_waker(&waker);
            f(Pin::new(&mut fut.0), &mut ctx)
        }
    }

    #[cfg(not(feature = "std-future"))]
    mod internal {
        use std::pin::Pin;
        use std::task::Context;

        use crate::task::noop_waker_ref;

        use super::*;

        pub(super) fn with_context<T, R, F>(fut: &mut IntoFuture<T>, f: F) -> R
        where
            T: Unpin,
            F: FnOnce(Pin<&mut T>, &mut Context<'_>) -> R,
        {
            let mut ctx = Context::from_waker(noop_waker_ref());
            f(Pin::new(&mut fut.0), &mut ctx)
        }
    }
}

#[cfg(feature = "std-future")]
mod impl_std {
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use futures_core::Future;

    use crate::response::Response;

    use super::IntoFuture;

    impl<T> Future for IntoFuture<T>
    where
        T: Response + Unpin,
    {
        type Output = Result<T::Ok, T::Error>;

        fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
            Response::poll(Pin::new(&mut self.0), ctx)
        }
    }
}
