//! A proof-of-concept crate to provides trait implementations of types in [`rusoto_codedeploy`],
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

use adventure::{response::Future01Response, BaseRequest, OneshotRequest, PagedRequest, Request};
use rusoto_codedeploy::*;
use rusoto_core::{RusotoError, RusotoFuture};

pub type RusotoResponse<T, E> = Future01Response<RusotoFuture<T, E>>;

/// A wrapper for various type of requests in [`rusoto_codedeploy`].
#[derive(Clone, Debug)]
pub struct AwsCodeDeploy<T> {
    inner: T,
}

pub trait AsCodeDeploy {
    type Output: CodeDeploy + ?Sized;
    fn as_code_deploy(&self) -> &Self::Output;
}

impl AsCodeDeploy for CodeDeployClient {
    type Output = CodeDeployClient;
    fn as_code_deploy(&self) -> &Self::Output {
        self
    }
}

impl<T: ?Sized> AsCodeDeploy for &T
where
    T: CodeDeploy,
{
    type Output = T;
    fn as_code_deploy(&self) -> &Self::Output {
        &**self
    }
}

impl<T: ?Sized> AsCodeDeploy for Box<T>
where
    T: CodeDeploy,
{
    type Output = T;
    fn as_code_deploy(&self) -> &Self::Output {
        &**self
    }
}

impl<T: ?Sized> AsCodeDeploy for Arc<T>
where
    T: CodeDeploy,
{
    type Output = T;
    fn as_code_deploy(&self) -> &Self::Output {
        &**self
    }
}

impl<T: ?Sized> AsCodeDeploy for Rc<T>
where
    T: CodeDeploy,
{
    type Output = T;
    fn as_code_deploy(&self) -> &Self::Output {
        &**self
    }
}

impl<P> AsCodeDeploy for Pin<P>
where
    P: Deref,
    <P as Deref>::Target: CodeDeploy,
{
    type Output = <P as Deref>::Target;
    fn as_code_deploy(&self) -> &Self::Output {
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

    (@ $wrapper:ident, $name:ident, $client:ident; type Ok = $ok:ty; type Error = $error:ty; $($rest:tt)*) => {
        impl BaseRequest for $wrapper<$name> {
            type Ok = $ok;
            type Error = RusotoError<$error>;
        }

        impl_adventure!(@@ $wrapper, $name, $client, $error; $($rest)*);
    };
    (@@ $wrapper:ident, $name:ident, $client:ident, $error:ty; ) => { };
    (@@ $wrapper:ident, $name:ident, $client:ident, $error:ty; retry: $variant:ident; $($rest:tt)*) => {
        impl RetriableRequest for $wrapper<$name> {
            fn should_retry(&self, err: &Self::Error, _next_interval: Duration) -> bool {
                if let RusotoError::Service($error::Server(_)) = err {
                    true
                } else {
                    false
                }
            }
        }

        impl_adventure!(@@ $wrapper, $name, $client, $error; $($rest)*);
    };
    (@@ $wrapper:ident, $name:ident, $client:ident, $error:ty; send: $method:ident; $($rest:tt)*) => {
        impl<C> OneshotRequest<C> for $wrapper<$name> where C: AsCodeDeploy {
            type Response = RusotoResponse<Self::Ok, $error>;

            fn send_once(self, client: C) -> Self::Response {
                Future01Response::new(client.as_code_deploy().$method(self.inner))
            }
        }

        impl<C> Request<C> for $wrapper<$name> where C: AsCodeDeploy {
            type Response = RusotoResponse<Self::Ok, $error>;

            #[inline]
            fn send(self: Pin<&mut Self>, client: C) -> Self::Response {
                self.clone().send_once(client)
            }
        }

        impl_adventure!(@@ $wrapper, $name, $client, $error; $($rest)*);
    };
    (@@ $wrapper:ident, $name:ident, $client:ident, $error:ty; advance: $token:ident; $($rest:tt)*) => {
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
    wrapper: AwsCodeDeploy;
    client: CodeDeploy;

    impl AddTagsToOnPremisesInstancesInput {
        type Ok = ();
        type Error = AddTagsToOnPremisesInstancesError;
        send: add_tags_to_on_premises_instances;
    }

    impl BatchGetApplicationRevisionsInput {
        type Ok = BatchGetApplicationRevisionsOutput;
        type Error = BatchGetApplicationRevisionsError;
        send: batch_get_application_revisions;
    }

    impl BatchGetApplicationsInput {
        type Ok = BatchGetApplicationsOutput;
        type Error = BatchGetApplicationsError;
        send: batch_get_applications;
    }

    impl BatchGetDeploymentGroupsInput {
        type Ok = BatchGetDeploymentGroupsOutput;
        type Error = BatchGetDeploymentGroupsError;
        send: batch_get_deployment_groups;
    }

    impl BatchGetDeploymentInstancesInput {
        type Ok = BatchGetDeploymentInstancesOutput;
        type Error = BatchGetDeploymentInstancesError;
        send: batch_get_deployment_instances;
    }

    impl BatchGetDeploymentTargetsInput {
        type Ok = BatchGetDeploymentTargetsOutput;
        type Error = BatchGetDeploymentTargetsError;
        send: batch_get_deployment_targets;
    }

    impl BatchGetDeploymentsInput {
        type Ok = BatchGetDeploymentsOutput;
        type Error = BatchGetDeploymentsError;
        send: batch_get_deployments;
    }

    impl BatchGetOnPremisesInstancesInput {
        type Ok = BatchGetOnPremisesInstancesOutput;
        type Error = BatchGetOnPremisesInstancesError;
        send: batch_get_on_premises_instances;
    }

    impl ContinueDeploymentInput {
        type Ok = ();
        type Error = ContinueDeploymentError;
        send: continue_deployment;
    }

    impl CreateApplicationInput {
        type Ok = CreateApplicationOutput;
        type Error = CreateApplicationError;
        send: create_application;
    }

    impl CreateDeploymentConfigInput {
        type Ok = CreateDeploymentConfigOutput;
        type Error = CreateDeploymentConfigError;
        send: create_deployment_config;
    }

    impl CreateDeploymentGroupInput {
        type Ok = CreateDeploymentGroupOutput;
        type Error = CreateDeploymentGroupError;
        send: create_deployment_group;
    }

    impl CreateDeploymentInput {
        type Ok = CreateDeploymentOutput;
        type Error = CreateDeploymentError;
        send: create_deployment;
    }

    impl DeleteApplicationInput {
        type Ok = ();
        type Error = DeleteApplicationError;
        send: delete_application;
    }

    impl DeleteDeploymentConfigInput {
        type Ok = ();
        type Error = DeleteDeploymentConfigError;
        send: delete_deployment_config;
    }

    impl DeleteDeploymentGroupInput {
        type Ok = DeleteDeploymentGroupOutput;
        type Error = DeleteDeploymentGroupError;
        send: delete_deployment_group;
    }

    impl DeleteGitHubAccountTokenInput {
        type Ok = DeleteGitHubAccountTokenOutput;
        type Error = DeleteGitHubAccountTokenError;
        send: delete_git_hub_account_token;
    }

    impl DeregisterOnPremisesInstanceInput {
        type Ok = ();
        type Error = DeregisterOnPremisesInstanceError;
        send: deregister_on_premises_instance;
    }

    impl GetApplicationInput {
        type Ok = GetApplicationOutput;
        type Error = GetApplicationError;
        send: get_application;
    }

    impl GetApplicationRevisionInput {
        type Ok = GetApplicationRevisionOutput;
        type Error = GetApplicationRevisionError;
        send: get_application_revision;
    }

    impl GetDeploymentConfigInput {
        type Ok = GetDeploymentConfigOutput;
        type Error = GetDeploymentConfigError;
        send: get_deployment_config;
    }

    impl GetDeploymentGroupInput {
        type Ok = GetDeploymentGroupOutput;
        type Error = GetDeploymentGroupError;
        send: get_deployment_group;
    }

    impl GetDeploymentInput {
        type Ok = GetDeploymentOutput;
        type Error = GetDeploymentError;
        send: get_deployment;
    }

    impl GetDeploymentInstanceInput {
        type Ok = GetDeploymentInstanceOutput;
        type Error = GetDeploymentInstanceError;
        send: get_deployment_instance;
    }

    impl GetDeploymentTargetInput {
        type Ok = GetDeploymentTargetOutput;
        type Error = GetDeploymentTargetError;
        send: get_deployment_target;
    }

    impl GetOnPremisesInstanceInput {
        type Ok = GetOnPremisesInstanceOutput;
        type Error = GetOnPremisesInstanceError;
        send: get_on_premises_instance;
    }

    impl ListApplicationRevisionsInput {
        type Ok = ListApplicationRevisionsOutput;
        type Error = ListApplicationRevisionsError;
        send: list_application_revisions;
        advance: next_token;
    }

    impl ListApplicationsInput {
        type Ok = ListApplicationsOutput;
        type Error = ListApplicationsError;
        send: list_applications;
        advance: next_token;
    }

    impl ListDeploymentConfigsInput {
        type Ok = ListDeploymentConfigsOutput;
        type Error = ListDeploymentConfigsError;
        send: list_deployment_configs;
        advance: next_token;
    }

    impl ListDeploymentGroupsInput {
        type Ok = ListDeploymentGroupsOutput;
        type Error = ListDeploymentGroupsError;
        send: list_deployment_groups;
        advance: next_token;
    }

    impl ListDeploymentInstancesInput {
        type Ok = ListDeploymentInstancesOutput;
        type Error = ListDeploymentInstancesError;
        send: list_deployment_instances;
        advance: next_token;
    }

    impl ListDeploymentTargetsInput {
        type Ok = ListDeploymentTargetsOutput;
        type Error = ListDeploymentTargetsError;
        send: list_deployment_targets;
        advance: next_token;
    }

    impl ListDeploymentsInput {
        type Ok = ListDeploymentsOutput;
        type Error = ListDeploymentsError;
        send: list_deployments;
        advance: next_token;
    }

    impl ListGitHubAccountTokenNamesInput {
        type Ok = ListGitHubAccountTokenNamesOutput;
        type Error = ListGitHubAccountTokenNamesError;
        send: list_git_hub_account_token_names;
        advance: next_token;
    }

    impl ListOnPremisesInstancesInput {
        type Ok = ListOnPremisesInstancesOutput;
        type Error = ListOnPremisesInstancesError;
        send: list_on_premises_instances;
        advance: next_token;
    }

    impl ListTagsForResourceInput {
        type Ok = ListTagsForResourceOutput;
        type Error = ListTagsForResourceError;
        send: list_tags_for_resource;
        advance: next_token;
    }

    impl PutLifecycleEventHookExecutionStatusInput {
        type Ok = PutLifecycleEventHookExecutionStatusOutput;
        type Error = PutLifecycleEventHookExecutionStatusError;
        send: put_lifecycle_event_hook_execution_status;
    }

    impl RegisterApplicationRevisionInput {
        type Ok = ();
        type Error = RegisterApplicationRevisionError;
        send: register_application_revision;
    }

    impl RegisterOnPremisesInstanceInput {
        type Ok = ();
        type Error = RegisterOnPremisesInstanceError;
        send: register_on_premises_instance;
    }

    impl RemoveTagsFromOnPremisesInstancesInput {
        type Ok = ();
        type Error = RemoveTagsFromOnPremisesInstancesError;
        send: remove_tags_from_on_premises_instances;
    }

    impl SkipWaitTimeForInstanceTerminationInput {
        type Ok = ();
        type Error = SkipWaitTimeForInstanceTerminationError;
        send: skip_wait_time_for_instance_termination;
    }

    impl StopDeploymentInput {
        type Ok = StopDeploymentOutput;
        type Error = StopDeploymentError;
        send: stop_deployment;
    }

    impl TagResourceInput {
        type Ok = TagResourceOutput;
        type Error = TagResourceError;
        send: tag_resource;
    }

    impl UntagResourceInput {
        type Ok = UntagResourceOutput;
        type Error = UntagResourceError;
        send: untag_resource;
    }

    impl UpdateApplicationInput {
        type Ok = ();
        type Error = UpdateApplicationError;
        send: update_application;
    }

    impl UpdateDeploymentGroupInput {
        type Ok = UpdateDeploymentGroupOutput;
        type Error = UpdateDeploymentGroupError;
        send: update_deployment_group;
    }

}
