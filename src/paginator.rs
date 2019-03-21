use std::pin::Pin;

use pin_utils::unsafe_pinned;

use crate::compat::{Poll, Waker};
use crate::request::PagedRequest;
use crate::response::Response;

/// A stream over the pages that consists the entire set from the request.
pub struct Paginator<C, R>
where
    R: PagedRequest<C>,
{
    client: C,
    request: Option<R>,
    next: Option<R::Response>,
}

impl<C, R> Paginator<C, R>
where
    R: PagedRequest<C>,
{
    unsafe_pinned!(request: Option<R>);

    unsafe_pinned!(next: Option<R::Response>);

    pub fn new(client: C, request: R) -> Self {
        Paginator {
            client,
            request: Some(request),
            next: None,
        }
    }
}

impl<C, R> Unpin for Paginator<C, R>
where
    C: Unpin,
    R: PagedRequest<C> + Unpin,
{
}

impl<C, R> Paginator<C, R>
where
    C: Clone,
    R: PagedRequest<C> + Unpin,
{
    fn poll_next(mut self: Pin<&mut Self>, waker: &Waker) -> Poll<Option<Result<R::Ok, R::Error>>> {
        if self.as_mut().next().is_none() {
            if let Some(request) = &self.as_ref().request {
                let next = request.send(self.client.clone());
                self.as_mut().next().set(Some(next));
            } else {
                return Poll::Ready(None);
            }
        };

        assert!(self.as_mut().next().is_some());
        assert!(self.as_mut().request().is_some());

        let page = match self.as_mut().next().as_pin_mut().unwrap().poll(waker) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Ok(x)) => x,
            Poll::Ready(Err(e)) => {
                self.as_mut().next().set(None);
                return Poll::Ready(Some(Err(e.into())));
            }
        };
        self.as_mut().next().set(None);

        let advanced = if let Some(mut r) = self.as_mut().request().as_pin_mut() {
            r.advance(&page)
        } else {
            true
        };
        if !advanced {
            self.as_mut().request().set(None);
        }

        Poll::Ready(Some(Ok(page)))
    }
}

#[cfg(all(feature = "futures01", not(feature = "std-future")))]
mod impl_futures01 {
    use std::pin::Pin;

    use futures::{Async, Poll, Stream};

    use super::Paginator;
    use crate::compat::Waker;
    use crate::request::PagedRequest;

    impl<C, R> Stream for Paginator<C, R>
    where
        C: Clone + Unpin,
        R: PagedRequest<C> + Unpin,
    {
        type Item = R::Ok;
        type Error = R::Error;

        fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
            use crate::compat::Poll::*;
            let w = unsafe { Waker::blank() };
            match Paginator::poll_next(Pin::new(self), &w) {
                Ready(Some(Ok(i))) => Ok(Async::Ready(Some(i))),
                Ready(Some(Err(e))) => Err(e),
                Ready(None) => Ok(Async::Ready(None)),
                Pending => Ok(Async::NotReady),
            }
        }
    }
}

#[cfg(feature = "std-future")]
mod impl_std {
    use std::pin::Pin;

    use futures_core::{task::Waker, Stream};

    use super::Paginator;
    use crate::compat::Poll;
    use crate::request::PagedRequest;

    impl<C, R> Stream for Paginator<C, R>
    where
        C: Clone,
        R: PagedRequest<C> + Unpin,
    {
        type Item = Result<R::Ok, R::Error>;

        fn poll_next(self: Pin<&mut Self>, w: &Waker) -> Poll<Option<Self::Item>> {
            Paginator::poll_next(self, w)
        }
    }
}
