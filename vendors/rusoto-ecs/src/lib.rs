//! A proof-of-concept crate to provides trait implementations of types in [`rusoto_ecs`],
//! to work with [`adventure`].
//!
//! Examples
//! --------
//!
//! ```ignore
//! let req = ListServicesRequest {
//!     cluster: Some("MyEcsCluster".to_owned()),
//!     ..Default::default()
//! };
//!
//! AwsEcs::from(req)
//!     .retry()
//!     .paginate(Arc::new(client))
use std::ops::Deref;
use std::time::Duration;

use adventure::{
    response::Future01Response, BaseRequest, OneshotRequest, PagedRequest, Request,
    RetriableRequest,
};
use rusoto_core::RusotoFuture;
use rusoto_ecs::*;

pub type RusotoResponse<R> =
    Future01Response<RusotoFuture<<R as BaseRequest>::Ok, <R as BaseRequest>::Error>>;

/// A wrapper for various type of requests in [`rusoto_ecs`].
#[derive(Clone, Debug)]
pub struct AwsEcs<T> {
    inner: T,
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

    (@ $wrapper:ident, $name:ident, $client:ident; ) => { };
    (@ $wrapper:ident, $name:ident, $client:ident; type Ok = $ok:ident; type Error = $error:ident; $($rest:tt)*) => {
        impl BaseRequest for $wrapper<$name> {
            type Ok = $ok;
            type Error = $error;
        }

        impl_adventure!(@@ $wrapper, $name, $client, $error; $($rest)*);
    };
    (@@ $wrapper:ident, $name:ident, $client:ident, $error:ident; retry: $variant:ident; $($rest:tt)*) => {
        impl RetriableRequest for $wrapper<$name> {
            fn should_retry(&self, err: &Self::Error, _next_interval: Duration) -> bool {
                if let $error::Server(_) = err {
                    true
                } else {
                    false
                }
            }
        }

        impl_adventure!(@ $wrapper, $name, $client; $($rest)*);
    };
    (@@ $wrapper:ident, $name:ident, $client:ident, $error:ident; $($rest:tt)*) => {
        impl_adventure!(@ $wrapper, $name, $client; $($rest)*);
    };
    (@ $wrapper:ident, $name:ident, $client:ident; send: $method:ident; $($rest:tt)*) => {
        impl<P> OneshotRequest<P> for $wrapper<$name> where P: Deref, <P as Deref>::Target: $client {
            type Response = RusotoResponse<Self>;

            fn send_once(self, client: P) -> Self::Response {
                Future01Response::new(client.$method(self.inner))
            }
        }

        impl<P> Request<P> for $wrapper<$name> where P: Deref, <P as Deref>::Target: $client {
            type Response = RusotoResponse<Self>;

            #[inline]
            fn send(&self, client: P) -> Self::Response {
                self.clone().send_once(client)
            }
        }

        impl_adventure!(@ $wrapper, $name, $client; $($rest)*);
    };
    (@ $wrapper:ident, $name:ident, $client:ident; advance: $token:ident; $($rest:tt)*) => {
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

        impl_adventure!(@ $wrapper, $name, $client; $($rest)*);
    }
}

impl_adventure! {
    wrapper: AwsEcs;
    client: Ecs;

    impl CreateClusterRequest {
        type Ok = CreateClusterResponse;
        type Error = CreateClusterError;
        retry: Server;
        send: create_cluster;
    }

    impl CreateServiceRequest {
        type Ok = CreateServiceResponse;
        type Error = CreateServiceError;
        retry: Server;
        send: create_service;
    }

    impl DeleteAccountSettingRequest {
        type Ok = DeleteAccountSettingResponse;
        type Error = DeleteAccountSettingError;
        retry: Server;
        send: delete_account_setting;
    }

    impl DeleteAttributesRequest {
        type Ok = DeleteAttributesResponse;
        type Error = DeleteAttributesError;
        send: delete_attributes;
    }

    impl DeleteClusterRequest {
        type Ok = DeleteClusterResponse;
        type Error = DeleteClusterError;
        retry: Server;
        send: delete_cluster;
    }

    impl DeleteServiceRequest {
        type Ok = DeleteServiceResponse;
        type Error = DeleteServiceError;
        retry: Server;
        send: delete_service;
    }

    impl DeregisterContainerInstanceRequest {
        type Ok = DeregisterContainerInstanceResponse;
        type Error = DeregisterContainerInstanceError;
        retry: Server;
        send: deregister_container_instance;
    }

    impl DeregisterTaskDefinitionRequest {
        type Ok = DeregisterTaskDefinitionResponse;
        type Error = DeregisterTaskDefinitionError;
        retry: Server;
        send: deregister_task_definition;
    }

    impl DescribeClustersRequest {
        type Ok = DescribeClustersResponse;
        type Error = DescribeClustersError;
        retry: Server;
        send: describe_clusters;
    }

    impl DescribeContainerInstancesRequest {
        type Ok = DescribeContainerInstancesResponse;
        type Error = DescribeContainerInstancesError;
        retry: Server;
        send: describe_container_instances;
    }

    impl DescribeServicesRequest {
        type Ok = DescribeServicesResponse;
        type Error = DescribeServicesError;
        retry: Server;
        send: describe_services;
    }

    impl DescribeTaskDefinitionRequest {
        type Ok = DescribeTaskDefinitionResponse;
        type Error = DescribeTaskDefinitionError;
        retry: Server;
        send: describe_task_definition;
    }

    impl DescribeTasksRequest {
        type Ok = DescribeTasksResponse;
        type Error = DescribeTasksError;
        retry: Server;
        send: describe_tasks;
    }

    impl DiscoverPollEndpointRequest {
        type Ok = DiscoverPollEndpointResponse;
        type Error = DiscoverPollEndpointError;
        retry: Server;
        send: discover_poll_endpoint;
    }

    impl ListAccountSettingsRequest {
        type Ok = ListAccountSettingsResponse;
        type Error = ListAccountSettingsError;
        retry: Server;
        send: list_account_settings;
        advance: next_token;
    }

    impl ListAttributesRequest {
        type Ok = ListAttributesResponse;
        type Error = ListAttributesError;
        send: list_attributes;
        advance: next_token;
    }

    impl ListClustersRequest {
        type Ok = ListClustersResponse;
        type Error = ListClustersError;
        retry: Server;
        send: list_clusters;
        advance: next_token;
    }

    impl ListContainerInstancesRequest {
        type Ok = ListContainerInstancesResponse;
        type Error = ListContainerInstancesError;
        retry: Server;
        send: list_container_instances;
        advance: next_token;
    }

    impl ListServicesRequest {
        type Ok = ListServicesResponse;
        type Error = ListServicesError;
        retry: Server;
        send: list_services;
        advance: next_token;
    }

    impl ListTagsForResourceRequest {
        type Ok = ListTagsForResourceResponse;
        type Error = ListTagsForResourceError;
        retry: Server;
        send: list_tags_for_resource;
    }

    impl ListTaskDefinitionFamiliesRequest {
        type Ok = ListTaskDefinitionFamiliesResponse;
        type Error = ListTaskDefinitionFamiliesError;
        retry: Server;
        send: list_task_definition_families;
        advance: next_token;
    }

    impl ListTaskDefinitionsRequest {
        type Ok = ListTaskDefinitionsResponse;
        type Error = ListTaskDefinitionsError;
        retry: Server;
        send: list_task_definitions;
        advance: next_token;
    }

    impl ListTasksRequest {
        type Ok = ListTasksResponse;
        type Error = ListTasksError;
        retry: Server;
        send: list_tasks;
        advance: next_token;
    }

    impl PutAccountSettingRequest {
        type Ok = PutAccountSettingResponse;
        type Error = PutAccountSettingError;
        retry: Server;
        send: put_account_setting;
    }

    impl PutAccountSettingDefaultRequest {
        type Ok = PutAccountSettingDefaultResponse;
        type Error = PutAccountSettingDefaultError;
        retry: Server;
        send: put_account_setting_default;
    }

    impl PutAttributesRequest {
        type Ok = PutAttributesResponse;
        type Error = PutAttributesError;
        send: put_attributes;
    }

    impl RegisterContainerInstanceRequest {
        type Ok = RegisterContainerInstanceResponse;
        type Error = RegisterContainerInstanceError;
        retry: Server;
        send: register_container_instance;
    }

    impl RegisterTaskDefinitionRequest {
        type Ok = RegisterTaskDefinitionResponse;
        type Error = RegisterTaskDefinitionError;
        retry: Server;
        send: register_task_definition;
    }

    impl RunTaskRequest {
        type Ok = RunTaskResponse;
        type Error = RunTaskError;
        retry: Server;
        send: run_task;
    }

    impl StartTaskRequest {
        type Ok = StartTaskResponse;
        type Error = StartTaskError;
        retry: Server;
        send: start_task;
    }

    impl StopTaskRequest {
        type Ok = StopTaskResponse;
        type Error = StopTaskError;
        retry: Server;
        send: stop_task;
    }

    impl SubmitContainerStateChangeRequest {
        type Ok = SubmitContainerStateChangeResponse;
        type Error = SubmitContainerStateChangeError;
        retry: Server;
        send: submit_container_state_change;
    }

    impl SubmitTaskStateChangeRequest {
        type Ok = SubmitTaskStateChangeResponse;
        type Error = SubmitTaskStateChangeError;
        retry: Server;
        send: submit_task_state_change;
    }

    impl TagResourceRequest {
        type Ok = TagResourceResponse;
        type Error = TagResourceError;
        retry: Server;
        send: tag_resource;
    }

    impl UntagResourceRequest {
        type Ok = UntagResourceResponse;
        type Error = UntagResourceError;
        retry: Server;
        send: untag_resource;
    }

    impl UpdateContainerAgentRequest {
        type Ok = UpdateContainerAgentResponse;
        type Error = UpdateContainerAgentError;
        retry: Server;
        send: update_container_agent;
    }

    impl UpdateContainerInstancesStateRequest {
        type Ok = UpdateContainerInstancesStateResponse;
        type Error = UpdateContainerInstancesStateError;
        retry: Server;
        send: update_container_instances_state;
    }

    impl UpdateServiceRequest {
        type Ok = UpdateServiceResponse;
        type Error = UpdateServiceError;
        retry: Server;
        send: update_service;
    }
}
