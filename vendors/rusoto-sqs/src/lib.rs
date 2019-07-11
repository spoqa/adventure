//! A proof-of-concept crate to provides trait implementations of types in [`rusoto_sqs`],
//! to work with [`adventure`].
//!
//! Examples
//! --------
//!
//! ```ignore
//! ```
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use adventure::{
    response::Future01Response, BaseRequest, OneshotRequest, PagedRequest, Request,
    RetriableRequest,
};
use futures::Future;
use rusoto_core::{RusotoError, RusotoFuture};
use rusoto_sqs::*;

pub type RusotoResponse<T, E> = Future01Response<RusotoFuture<T, E>>;

/// A wrapper for various type of requests in [`rusoto_sqs`].
#[derive(Clone, Debug)]
pub struct AwsSqs<T> {
    inner: T,
}

pub trait AsSqs {
    type Output: Sqs + ?Sized;
    fn as_sqs(&self) -> &Self::Output;
}

impl AsSqs for SqsClient {
    type Output = SqsClient;
    fn as_sqs(&self) -> &SqsClient {
        self
    }
}

impl<T: ?Sized> AsSqs for &T
where
    T: Sqs,
{
    type Output = T;
    fn as_sqs(&self) -> &Self::Output {
        &**self
    }
}

impl<T: ?Sized> AsSqs for Box<T>
where
    T: Sqs,
{
    type Output = T;
    fn as_sqs(&self) -> &Self::Output {
        &**self
    }
}

impl<T: ?Sized> AsSqs for Arc<T>
where
    T: Sqs,
{
    type Output = T;
    fn as_sqs(&self) -> &Self::Output {
        &**self
    }
}

impl<T: ?Sized> AsSqs for Rc<T>
where
    T: Sqs,
{
    type Output = T;
    fn as_sqs(&self) -> &Self::Output {
        &**self
    }
}

impl<P> AsSqs for Pin<P>
where
    P: Deref,
    <P as Deref>::Target: Sqs,
{
    type Output = <P as Deref>::Target;
    fn as_sqs(&self) -> &Self::Output {
        &**self
    }
}

macro_rules! impl_adventure {
    {
        wrapper: $wrapper:ident;
        client: $client:ident;
        $( impl $name:ident { $($body:tt)* } )*
    } => {
        $(
            impl From<$name> for $wrapper<$name> {
                fn from(req: $name) -> Self {
                    $wrapper { inner: req }
                }
            }

            impl_adventure!(@ $wrapper, $name, $client; $($body)*);
        )*
    };

    (@ $wrapper:ident, $name:ident, $client:ident; type Ok = $ok:ty; type Error = $error:ident; $($rest:tt)*) => {
        impl BaseRequest for $wrapper<$name> {
            type Ok = $ok;
            type Error = RusotoError<$error>;
        }

        impl_adventure!(@@ $wrapper, $name, $client, $error; $($rest)*);
    };
    (@@ $wrapper:ident, $name:ident, $client:ident, $error:ident; ) => { };
    (@@ $wrapper:ident, $name:ident, $client:ident, $error:ident; retry: $variant:ident; $($rest:tt)*) => {
        impl RetriableRequest for $wrapper<$name> {
            fn should_retry(&self, err: &Self::Error, _next_interval: Duration) -> bool {
                if let RusotoError::HttpDispatch(_) = err {
                    true
                } else {
                    false
                }
            }
        }

        impl_adventure!(@@ $wrapper, $name, $client, $error; $($rest)*);
    };
    (@@ $wrapper:ident, $name:ident, $client:ident, $error:ident; send: $method:ident; $($rest:tt)*) => {
        impl<C> OneshotRequest<C> for $wrapper<$name> where C: AsSqs {
            type Response = RusotoResponse<Self::Ok, $error>;

            fn send_once(self, client: C) -> Self::Response {
                Future01Response::new(client.as_sqs().$method(self.inner))
            }
        }

        impl<C> Request<C> for $wrapper<$name> where C: AsSqs {
            type Response = <Self as OneshotRequest<C>>::Response;

            #[inline]
            fn send(self: Pin<&mut Self>, client: C) -> Self::Response {
                self.clone().send_once(client)
            }
        }

        impl_adventure!(@@ $wrapper, $name, $client, $error; $($rest)*);
    };
    (@@ $wrapper:ident, $name:ident, $client:ident, $error:ident; advance: $token:ident; $($rest:tt)*) => {
        impl PagedRequest for $wrapper<$name> {
            fn advance(&mut self, response: &Self::Ok) -> bool {
                if let Some(next) = response.$token.clone() {
                    self.inner.$token = Some(next);
                    true
                } else {
                    false
                }
            }
        }

        impl_adventure!(@@ $wrapper, $name, $client, $error; $($rest)*);
    }
}

impl_adventure! {
    wrapper: AwsSqs;
    client: Sqs;

    impl AddPermissionRequest {
        type Ok = ();
        type Error = AddPermissionError;
        retry: Server;
        send: add_permission;
    }

    impl ChangeMessageVisibilityRequest {
        type Ok = ();
        type Error = ChangeMessageVisibilityError;
        retry: Server;
        send: change_message_visibility;
    }

    impl ChangeMessageVisibilityBatchRequest {
        type Ok = ChangeMessageVisibilityBatchResult;
        type Error = ChangeMessageVisibilityBatchError;
        retry: Server;
        send: change_message_visibility_batch;
    }

    impl CreateQueueRequest {
        type Ok = CreateQueueResult;
        type Error = CreateQueueError;
        retry: Server;
        send: create_queue;
    }

    impl DeleteMessageRequest {
        type Ok = ();
        type Error = DeleteMessageError;
        retry: Server;
        send: delete_message;
    }

    impl DeleteMessageBatchRequest {
        type Ok = DeleteMessageBatchResult;
        type Error = DeleteMessageBatchError;
        retry: Server;
        send: delete_message_batch;
    }

    impl DeleteQueueRequest {
        type Ok = ();
        type Error = DeleteQueueError;
        retry: Server;
        send: delete_queue;
    }

    impl GetQueueAttributesRequest {
        type Ok = GetQueueAttributesResult;
        type Error = GetQueueAttributesError;
        retry: Server;
        send: get_queue_attributes;
    }

    impl GetQueueUrlRequest {
        type Ok = GetQueueUrlResult;
        type Error = GetQueueUrlError;
        retry: Server;
        send: get_queue_url;
    }

    impl ListDeadLetterSourceQueuesRequest {
        type Ok = ListDeadLetterSourceQueuesResult;
        type Error = ListDeadLetterSourceQueuesError;
        retry: Server;
        send: list_dead_letter_source_queues;
    }

    impl ListQueueTagsRequest {
        type Ok = ListQueueTagsResult;
        type Error = ListQueueTagsError;
        retry: Server;
        send: list_queue_tags;
    }

    impl ListQueuesRequest {
        type Ok = ListQueuesResult;
        type Error = ListQueuesError;
        retry: Server;
        send: list_queues;
    }

    impl PurgeQueueRequest {
        type Ok = ();
        type Error = PurgeQueueError;
        retry: Server;
        send: purge_queue;
    }

    impl ReceiveMessageRequest {
        type Ok = Vec<Message>;
        type Error = ReceiveMessageError;
    }

    impl RemovePermissionRequest {
        type Ok = ();
        type Error = RemovePermissionError;
        retry: Server;
        send: remove_permission;
    }

    impl SendMessageRequest {
        type Ok = SendMessageResult;
        type Error = SendMessageError;
        retry: Server;
        send: send_message;
    }

    impl SendMessageBatchRequest {
        type Ok = SendMessageBatchResult;
        type Error = SendMessageBatchError;
        retry: Server;
        send: send_message_batch;
    }

    impl SetQueueAttributesRequest {
        type Ok = ();
        type Error = SetQueueAttributesError;
        retry: Server;
        send: set_queue_attributes;
    }

    impl TagQueueRequest {
        type Ok = ();
        type Error = TagQueueError;
        retry: Server;
        send: tag_queue;
    }

    impl UntagQueueRequest {
        type Ok = ();
        type Error = UntagQueueError;
        retry: Server;
        send: untag_queue;
    }
}

impl<C> OneshotRequest<C> for AwsSqs<ReceiveMessageRequest>
where
    C: AsSqs,
{
    type Response =
        Future01Response<Box<dyn Future<Item = Self::Ok, Error = Self::Error> + Send + 'static>>;

    fn send_once(self, client: C) -> Self::Response {
        let f = client
            .as_sqs()
            .receive_message(self.inner)
            .map(|res| res.messages.unwrap_or_else(Vec::new));
        Future01Response::new(Box::new(f))
    }
}

impl<C> Request<C> for AwsSqs<ReceiveMessageRequest>
where
    C: AsSqs,
{
    type Response = <Self as OneshotRequest<C>>::Response;

    #[inline]
    fn send(self: Pin<&mut Self>, client: C) -> Self::Response {
        self.clone().send_once(client)
    }
}

impl RetriableRequest for AwsSqs<ReceiveMessageRequest> {
    fn should_retry(&self, err: &Self::Error, _next_interval: Duration) -> bool {
        match err {
            RusotoError::HttpDispatch(_)
            | RusotoError::Service(ReceiveMessageError::OverLimit(_)) => true,
            _ => false,
        }
    }
}

impl PagedRequest for AwsSqs<ReceiveMessageRequest> {
    fn advance(&mut self, response: &Self::Ok) -> bool {
        if let Some(id) = self.inner.receive_request_attempt_id.as_mut() {
            use sha2::Digest;
            use std::fmt::Write;
            let mut hasher = sha2::Sha256::new();
            hasher.input(id.as_bytes());
            hasher.input(b"next");
            if let Some(i) = response.first().and_then(|m| m.message_id.as_ref()) {
                hasher.input(i.as_bytes());
            }
            id.clear();
            write!(id, "{:x}", hasher.result()).unwrap();
        }
        true
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn receive_message_paged_advance() {
        let mut req = AwsSqs::from(ReceiveMessageRequest {
            receive_request_attempt_id: Some("first".to_owned()),
            ..Default::default()
        });
        assert!(req.advance(&vec![]));
        let id = req.inner.receive_request_attempt_id.unwrap();
        println!("{}", id);
        assert_eq!(id.len(), 64);
        assert!(regex::Regex::new(r"[0-9a-z]{2}+").unwrap().is_match(&id));
    }
}
