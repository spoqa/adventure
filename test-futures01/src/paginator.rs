#![cfg(test)]

use std::sync::atomic::{AtomicUsize, Ordering};

use futures::{future, prelude::*, Future};
use tokio::runtime::current_thread::block_on_all as block_on;

use adventure::{
    paginator::Paginator,
    prelude::*,
    request::{PagedRequest, Request},
    response::ResponseFuture,
};

struct MockClient {
    called: AtomicUsize,
    pred: Box<dyn Fn(&Numbers) -> bool>,
}

#[cfg(test)]
impl MockClient {
    fn new(pred: impl Fn(&Numbers) -> bool + 'static) -> MockClient {
        MockClient {
            called: Default::default(),
            pred: Box::new(pred),
        }
    }
}

#[derive(Debug, Default)]
struct Numbers {
    current: AtomicUsize,
    end: usize,
}

impl Request<&MockClient> for &Numbers {
    type Ok = usize;
    type Error = ();
    type Response =
        ResponseFuture<Box<dyn Future<Item = usize, Error = ()> + Send + Sync + 'static>>;

    fn send(&self, client: &MockClient) -> Self::Response {
        client.called.fetch_add(1, Ordering::SeqCst);
        if (client.pred)(self) {
            ResponseFuture::from(Box::new(future::ok(self.current.load(Ordering::SeqCst))) as Box<_>)
        } else {
            ResponseFuture::from(Box::new(future::err(())) as Box<_>)
        }
    }
}

impl PagedRequest<&MockClient> for &Numbers {
    fn advance(&mut self, response: &<Self::Response as Response>::Ok) -> bool {
        if *response < self.end {
            self.current.fetch_add(1, Ordering::SeqCst);
            true
        } else {
            false
        }
    }
}

#[test]
fn paginator_basic() {
    let client = MockClient::new(|_| true);
    let numbers = Numbers {
        current: AtomicUsize::new(1),
        end: 5,
    };
    let paginator = Paginator::new(&client, &numbers);

    let responses = block_on(paginator.collect());
    assert_eq!(Ok(vec![1, 2, 3, 4, 5]), responses);
    assert_eq!(numbers.current.load(Ordering::SeqCst), 5);
    assert_eq!(numbers.end, 5);
    assert_eq!(client.called.load(Ordering::SeqCst), 5);
}

#[test]
fn paginator_basic_2() {
    let client = MockClient::new(|n| n.current.load(Ordering::SeqCst) < 7);
    let numbers = Numbers {
        current: AtomicUsize::new(1),
        end: 20,
    };
    let paginator = Paginator::new(&client, &numbers);

    let responses = block_on(paginator.collect());
    assert_eq!(Err(()), responses);
    assert_eq!(numbers.current.load(Ordering::SeqCst), 7);
    assert_eq!(client.called.load(Ordering::SeqCst), 7);
}

#[cfg(test)]
fn block_on_next<S>(stream: &mut Option<S>) -> Option<Result<S::Item, S::Error>>
where
    S: Stream,
{
    let (r, s) = match block_on(stream.take()?.into_future()) {
        Ok((Some(i), s)) => (Some(Ok(i)), s),
        Ok((None, s)) => (None, s),
        Err((e, s)) => (Some(Err(e)), s),
    };
    stream.replace(s);
    r
}

#[test]
fn paginator_step() {
    let client = MockClient::new(|_| true);
    let numbers = Numbers {
        current: AtomicUsize::new(1),
        end: 3,
    };
    let mut paginator = Some(Paginator::new(&client, &numbers));

    assert_eq!(block_on_next(&mut paginator), Some(Ok(1)));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 2);
    assert_eq!(client.called.load(Ordering::SeqCst), 1);

    assert_eq!(block_on_next(&mut paginator), Some(Ok(2)));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 2);

    assert_eq!(block_on_next(&mut paginator), Some(Ok(3)));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 3);

    assert_eq!(block_on_next(&mut paginator), None);
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 3);

    assert_eq!(block_on_next(&mut paginator), None);
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 3);
}

#[test]
fn paginator_step_with_error() {
    let client = MockClient::new(|n| n.current.load(Ordering::SeqCst) < 3);
    let numbers = Numbers {
        current: AtomicUsize::new(1),
        end: 3,
    };
    let mut paginator = Some(Paginator::new(&client, &numbers));

    assert_eq!(block_on_next(&mut paginator), Some(Ok(1)));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 2);
    assert_eq!(client.called.load(Ordering::SeqCst), 1);

    assert_eq!(block_on_next(&mut paginator), Some(Ok(2)));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 2);

    assert_eq!(block_on_next(&mut paginator), Some(Err(())));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 3);

    assert_eq!(block_on_next(&mut paginator), Some(Err(())));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 4);
}
