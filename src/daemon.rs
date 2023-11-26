// Daemon module
// watch for changes in all LoadBalancer services and update the IP addresses

use color_eyre::Result;
use futures::StreamExt;
use k8s_openapi::api::{apps::v1::Deployment, core::v1::Service};
use kube::{
    api::{Api, ListParams, Patch, PatchParams, ResourceExt},
    runtime::{controller::Action, watcher::Config, Controller},
    Client,
};
use std::sync::Arc;

use std::time::Duration;
use tracing::{debug, error, info, instrument};

use crate::ops::ExitNode;
use crate::{deployment::create_owned_deployment, error::ReconcileError};

// pub fn get_trace_id() -> opentelemetry::trace::TraceId {
//     // opentelemetry::Context -> opentelemetry::trace::Span
//     use opentelemetry::trace::TraceContextExt as _;
//     // tracing::Span -> opentelemetry::Context
//     use tracing_opentelemetry::OpenTelemetrySpanExt as _;

//     tracing::Span::current()
//         .context()
//         .span()
//         .span_context()
//         .trace_id()
// }

struct Context {
    client: Client,
}

// #[instrument(skip(ctx), fields(trace_id))]
/// Reconcile cluster state
#[instrument(skip(ctx))]
async fn reconcile(obj: Arc<Service>, ctx: Arc<Context>) -> Result<Action, ReconcileError> {
    // let trace_id = get_trace_id();
    // Span::current().record("trace_id", &field::display(&trace_id));

    // Return if service is not LoadBalancer or if the loadBalancerClass is not blank or set to "chisel-operator.io/chisel-operator-class"
    if obj
        .spec
        .as_ref()
        .filter(|spec| spec.type_ == Some("LoadBalancer".to_string()))
        .is_none()
        || obj
            .spec
            .as_ref()
            .filter(|spec| {
                spec.load_balancer_class.is_none()
                    || spec.load_balancer_class
                        == Some("chisel-operator.io/chisel-operator-class".to_string())
            })
            .is_none()
    {
        return Ok(Action::await_change());
    }

    info!("reconcile request: {}", obj.name_any());

    // We can unwrap safely since Service is namespaced scoped
    let services: Api<Service> = Api::namespaced(ctx.client.clone(), &obj.namespace().unwrap());

    // We will only be supporting 1 exit node for all services for now, so we can just grab the first one
    let nodes: Api<ExitNode> = Api::all(ctx.client.clone());
    let node_list = nodes.list(&ListParams::default().timeout(30)).await?;
    let node = node_list
        .items
        .first()
        .ok_or(ReconcileError::NoAvailableExitNodes)?;

    tracing::debug!("node: {:?}", node);

    // We can unwrap safely since ExitNode is namespaced scoped
    let deployments: Api<Deployment> =
        Api::namespaced(ctx.client.clone(), &node.namespace().unwrap());

    // TODO: We should refactor this such that each deployment of Chisel corresponds to an exit node
    // Currently each deployment of Chisel corresponds to a service, which means duplicate deployments of Chisel
    // This also caused some issues, where we (intuitively) made the owner ref of the deployment the service
    // which breaks since a service can be in a seperate namespace from the deployment (k8s disallows this)
    let deployment_data = create_owned_deployment(&obj, &node)?;
    let serverside = PatchParams::apply("chisel-operator").validation_strict();
    let _deployment = deployments
        .patch(
            &deployment_data.name_any(),
            &serverside,
            &Patch::Apply(deployment_data),
        )
        .await?;

    tracing::trace!("deployment: {:?}", _deployment);

    // new update: We now have the ability to use an external hostname for the exit node!
    // this means that you can now route traffic to the exit node using any network interface, or a domain if you wanted to
    // this is useful if you don't wanna open up the chisel API to the public internet
    let ip_address = node.spec.get_external_host();
    // Update the status for the LoadBalancer service
    // The ExitNode IP will always be set, so it is safe to unwrap the host
    let status_data = serde_json::json!({"status": {
        "loadBalancer": {
            "ingress": [
                {
                    "ip": ip_address
                }
            ]
        }
    }});
    debug!("Patching status for {}", obj.name_any());
    let _svcs = services
        .patch_status(
            // We can unwrap safely since Service is guaranteed to have a name
            obj.name_any().as_str(),
            &serverside.clone(),
            &Patch::Merge(status_data.clone()),
        )
        .await?;

    info!(status = ?status_data, "Patched status for {}", obj.name_any());

    Ok(Action::requeue(Duration::from_secs(3600)))
}

fn error_policy(_object: Arc<Service>, err: &ReconcileError, _ctx: Arc<Context>) -> Action {
    error!(err = ?err);
    Action::requeue(Duration::from_secs(5))
}

/// watches for Kubernetes service resources and runs a controller to reconcile them.
pub async fn run() -> color_eyre::Result<()> {
    let client = Client::try_default().await?;
    // watch for K8s service resources (default)
    let services: Api<Service> = Api::all(client.clone());

    Controller::new(services, Config::default())
        .run(reconcile, error_policy, Arc::new(Context { client }))
        .for_each(|_| futures::future::ready(()))
        .await;

    Ok(())
}
