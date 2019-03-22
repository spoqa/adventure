#![cfg(feature = "std-future-test")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures_util::{future, try_future::TryFutureExt};
use tokio::runtime::current_thread::block_on_all;

use crate::prelude::*;
use crate::response::{Response, ResponseStdFutureObj};
use crate::retry::WithBackoff;

#[derive(Debug, Default)]
pub(crate) struct Numbers {
    current: AtomicUsize,
    end: usize,
}

impl Clone for Numbers {
    fn clone(&self) -> Self {
        Numbers {
            current: AtomicUsize::new(self.current.load(Ordering::SeqCst)),
            end: self.end,
        }
    }
}

type Resp = ResponseStdFutureObj<'static, usize, String>;

impl<C> Request<C> for Numbers {
    type Ok = usize;
    type Error = String;
    type Response = Resp;

    fn into_response(self, client: C) -> Self::Response {
        self.send(client)
    }
}

impl<C> RepeatableRequest<C> for Numbers {
    fn send(&self, _client: C) -> Self::Response {
        let i = self.current.fetch_add(1, Ordering::SeqCst);
        if i < self.end {
            ResponseStdFutureObj::new(future::err(format!("{} tried", i)))
        } else {
            ResponseStdFutureObj::new(future::ok(i))
        }
    }
}

impl<C> RetriableRequest<C> for Numbers {
    fn should_retry(&self, _error: &Self::Error, _next_interval: Duration) -> bool {
        true
    }
}

fn block_on<R>(req: R) -> Result<R::Ok, R::Error>
where
    R: Response + Unpin,
{
    block_on_all(req.into_future().compat())
}

#[test]
fn retry_simple() {
    let numbers = Numbers {
        current: AtomicUsize::new(1),
        end: 5,
    };
    let req = WithBackoff::with_retry(&numbers);

    assert_eq!(block_on(req.send(())).unwrap(), 5);
}
