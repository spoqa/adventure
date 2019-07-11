//! A proof-of-concept crate to provides trait implementations of types in [`rusoto_sns`],
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
use rusoto_core::{RusotoError, RusotoFuture};
use rusoto_sns::*;

pub type RusotoResponse<T, E> = Future01Response<RusotoFuture<T, E>>;

/// A wrapper for various type of requests in [`rusoto_sns`].
#[derive(Clone, Debug)]
pub struct AwsSns<T> {
    inner: T,
}

pub trait AsSns {
    type Output: Sns + ?Sized;
    fn as_sns(&self) -> &Self::Output;
}

impl AsSns for SnsClient {
    type Output = SnsClient;
    fn as_sns(&self) -> &SnsClient {
        self
    }
}

impl<T: ?Sized> AsSns for &T
where
    T: Sns,
{
    type Output = T;
    fn as_sns(&self) -> &Self::Output {
        &**self
    }
}

impl<T: ?Sized> AsSns for Box<T>
where
    T: Sns,
{
    type Output = T;
    fn as_sns(&self) -> &Self::Output {
        &**self
    }
}

impl<T: ?Sized> AsSns for Arc<T>
where
    T: Sns,
{
    type Output = T;
    fn as_sns(&self) -> &Self::Output {
        &**self
    }
}

impl<T: ?Sized> AsSns for Rc<T>
where
    T: Sns,
{
    type Output = T;
    fn as_sns(&self) -> &Self::Output {
        &**self
    }
}

impl<P> AsSns for Pin<P>
where
    P: Deref,
    <P as Deref>::Target: Sns,
{
    type Output = <P as Deref>::Target;
    fn as_sns(&self) -> &Self::Output {
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
        impl<C> OneshotRequest<C> for $wrapper<$name> where C: AsSns {
            type Response = RusotoResponse<Self::Ok, $error>;

            fn send_once(self, client: C) -> Self::Response {
                Future01Response::new(client.as_sns().$method(self.inner))
            }
        }

        impl<C> Request<C> for $wrapper<$name> where C: AsSns {
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
    wrapper: AwsSns;
    client: Sns;

    impl AddPermissionInput {
        type Ok = ();
        type Error = AddPermissionError;
        retry: Server;
        send: add_permission;
    }

    impl CheckIfPhoneNumberIsOptedOutInput {
        type Ok = CheckIfPhoneNumberIsOptedOutResponse;
        type Error = CheckIfPhoneNumberIsOptedOutError;
        retry: Server;
        send: check_if_phone_number_is_opted_out;
    }

    impl ConfirmSubscriptionInput {
        type Ok = ConfirmSubscriptionResponse;
        type Error = ConfirmSubscriptionError;
        retry: Server;
        send: confirm_subscription;
    }

    impl CreatePlatformApplicationInput {
        type Ok = CreatePlatformApplicationResponse;
        type Error = CreatePlatformApplicationError;
        retry: Server;
        send: create_platform_application;
    }

    impl CreatePlatformEndpointInput {
        type Ok = CreateEndpointResponse;
        type Error = CreatePlatformEndpointError;
        retry: Server;
        send: create_platform_endpoint;
    }

    impl CreateTopicInput {
        type Ok = CreateTopicResponse;
        type Error = CreateTopicError;
        retry: Server;
        send: create_topic;
    }

    impl DeleteEndpointInput {
        type Ok = ();
        type Error = DeleteEndpointError;
        retry: Server;
        send: delete_endpoint;
    }

    impl DeletePlatformApplicationInput {
        type Ok = ();
        type Error = DeletePlatformApplicationError;
        retry: Server;
        send: delete_platform_application;
    }

    impl DeleteTopicInput {
        type Ok = ();
        type Error = DeleteTopicError;
        retry: Server;
        send: delete_topic;
    }

    impl GetEndpointAttributesInput {
        type Ok = GetEndpointAttributesResponse;
        type Error = GetEndpointAttributesError;
        retry: Server;
        send: get_endpoint_attributes;
    }

    impl GetPlatformApplicationAttributesInput {
        type Ok = GetPlatformApplicationAttributesResponse;
        type Error = GetPlatformApplicationAttributesError;
        retry: Server;
        send: get_platform_application_attributes;
    }

    impl GetSMSAttributesInput {
        type Ok = GetSMSAttributesResponse;
        type Error = GetSMSAttributesError;
        retry: Server;
        send: get_sms_attributes;
    }

    impl GetSubscriptionAttributesInput {
        type Ok = GetSubscriptionAttributesResponse;
        type Error = GetSubscriptionAttributesError;
        retry: Server;
        send: get_subscription_attributes;
    }

    impl GetTopicAttributesInput {
        type Ok = GetTopicAttributesResponse;
        type Error = GetTopicAttributesError;
        retry: Server;
        send: get_topic_attributes;
    }

    impl ListEndpointsByPlatformApplicationInput {
        type Ok = ListEndpointsByPlatformApplicationResponse;
        type Error = ListEndpointsByPlatformApplicationError;
        retry: Server;
        send: list_endpoints_by_platform_application;
        advance: next_token;
    }

    impl ListPhoneNumbersOptedOutInput {
        type Ok = ListPhoneNumbersOptedOutResponse;
        type Error = ListPhoneNumbersOptedOutError;
        retry: Server;
        send: list_phone_numbers_opted_out;
        advance: next_token;
    }

    impl ListPlatformApplicationsInput {
        type Ok = ListPlatformApplicationsResponse;
        type Error = ListPlatformApplicationsError;
        retry: Server;
        send: list_platform_applications;
        advance: next_token;
    }

    impl ListSubscriptionsInput {
        type Ok = ListSubscriptionsResponse;
        type Error = ListSubscriptionsError;
        retry: Server;
        send: list_subscriptions;
        advance: next_token;
    }

    impl ListSubscriptionsByTopicInput {
        type Ok = ListSubscriptionsByTopicResponse;
        type Error = ListSubscriptionsByTopicError;
        retry: Server;
        send: list_subscriptions_by_topic;
        advance: next_token;
    }

    impl ListTagsForResourceRequest {
        type Ok = ListTagsForResourceResponse;
        type Error = ListTagsForResourceError;
        retry: Server;
        send: list_tags_for_resource;
    }

    impl ListTopicsInput {
        type Ok = ListTopicsResponse;
        type Error = ListTopicsError;
        retry: Server;
        send: list_topics;
        advance: next_token;
    }

    impl OptInPhoneNumberInput {
        type Ok = OptInPhoneNumberResponse;
        type Error = OptInPhoneNumberError;
        retry: Server;
        send: opt_in_phone_number;
    }

    impl PublishInput {
        type Ok = PublishResponse;
        type Error = PublishError;
        retry: Server;
        send: publish;
    }

    impl RemovePermissionInput {
        type Ok = ();
        type Error = RemovePermissionError;
        retry: Server;
        send: remove_permission;
    }

    impl SetEndpointAttributesInput {
        type Ok = ();
        type Error = SetEndpointAttributesError;
        retry: Server;
        send: set_endpoint_attributes;
    }

    impl SetPlatformApplicationAttributesInput {
        type Ok = ();
        type Error = SetPlatformApplicationAttributesError;
        retry: Server;
        send: set_platform_application_attributes;
    }

    impl SetSMSAttributesInput {
        type Ok = SetSMSAttributesResponse;
        type Error = SetSMSAttributesError;
        retry: Server;
        send: set_sms_attributes;
    }

    impl SetSubscriptionAttributesInput {
        type Ok = ();
        type Error = SetSubscriptionAttributesError;
        retry: Server;
        send: set_subscription_attributes;
    }

    impl SetTopicAttributesInput {
        type Ok = ();
        type Error = SetTopicAttributesError;
        retry: Server;
        send: set_topic_attributes;
    }

    impl SubscribeInput {
        type Ok = SubscribeResponse;
        type Error = SubscribeError;
        retry: Server;
        send: subscribe;
    }

    impl TagResourceRequest {
        type Ok = TagResourceResponse;
        type Error = TagResourceError;
        retry: Server;
        send: tag_resource;
    }

    impl UnsubscribeInput {
        type Ok = ();
        type Error = UnsubscribeError;
        retry: Server;
        send: unsubscribe;
    }

    impl UntagResourceRequest {
        type Ok = UntagResourceResponse;
        type Error = UntagResourceError;
        retry: Server;
        send: untag_resource;
    }

}
