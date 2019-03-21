#![cfg(test)]

use std::sync::atomic::{AtomicUsize, Ordering};

use futures_core::future::FutureObj;
use futures_executor::block_on;
use futures_util::{
    future::{self, FutureExt},
    stream::StreamExt,
    try_stream::TryStreamExt,
};

use adventure::{
    paginator::Paginator,
    prelude::*,
    request::{PagedRequest, Request},
    response::ResponseStdFuture,
};

struct MockClient {
    called: AtomicUsize,
    pred: Box<dyn Fn(&Numbers) -> bool>,
}

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
    type Response = ResponseStdFuture<FutureObj<'static, Result<usize, ()>>>;

    fn send(&self, client: &MockClient) -> Self::Response {
        client.called.fetch_add(1, Ordering::SeqCst);
        if (client.pred)(self) {
            FutureObj::new(future::ok(self.current.load(Ordering::SeqCst)).boxed()).into()
        } else {
            FutureObj::new(future::err(()).boxed()).into()
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

    let responses = block_on(paginator.try_collect::<Vec<_>>());
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

    let responses = block_on(paginator.try_collect::<Vec<_>>());
    assert_eq!(Err(()), responses);
    assert_eq!(numbers.current.load(Ordering::SeqCst), 7);
    assert_eq!(client.called.load(Ordering::SeqCst), 7);
}

#[test]
fn paginator_step() {
    let client = MockClient::new(|_| true);
    let numbers = Numbers {
        current: AtomicUsize::new(1),
        end: 3,
    };
    let mut paginator = Paginator::new(&client, &numbers);

    assert_eq!(block_on(paginator.next()), Some(Ok(1)));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 2);
    assert_eq!(client.called.load(Ordering::SeqCst), 1);

    assert_eq!(block_on(paginator.next()), Some(Ok(2)));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 2);

    assert_eq!(block_on(paginator.next()), Some(Ok(3)));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 3);

    assert_eq!(block_on(paginator.next()), None);
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 3);

    assert_eq!(block_on(paginator.next()), None);
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
    let mut paginator = Paginator::new(&client, &numbers);

    assert_eq!(block_on(paginator.next()), Some(Ok(1)));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 2);
    assert_eq!(client.called.load(Ordering::SeqCst), 1);

    assert_eq!(block_on(paginator.next()), Some(Ok(2)));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 2);

    assert_eq!(block_on(paginator.next()), Some(Err(())));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 3);

    assert_eq!(block_on(paginator.next()), Some(Err(())));
    assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
    assert_eq!(client.called.load(Ordering::SeqCst), 4);
}
