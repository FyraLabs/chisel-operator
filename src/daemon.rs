// Daemon module
// watch for changes in all LoadBalancer services and update the IP addresses

use color_eyre::Result;
use futures::StreamExt;
use k8s_openapi::api::{apps::v1::Deployment, core::v1::Service};
use kube::{
    api::{Api, ListParams, Patch, PatchParams, ResourceExt},
    runtime::{controller::Action, watcher::Config, Controller},
    Client, Resource,
};
use std::sync::Arc;

use std::time::Duration;
use thiserror::Error;
use tracing::{info, instrument};

use crate::deployment::create_owned_deployment;
use crate::ops::ExitNode;

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

#[derive(Error, Debug)]
pub enum ReconcileError {}

// #[instrument(skip(_ctx), fields(trace_id))]
#[instrument(skip(_ctx))]
async fn reconcile(obj: Arc<Service>, _ctx: Arc<()>) -> Result<Action, ReconcileError> {
    // let trace_id = get_trace_id();
    // Span::current().record("trace_id", &field::display(&trace_id));

    if obj
        .spec
        .as_ref()
        .filter(|spec| spec.type_ == Some("LoadBalancer".to_string()))
        .is_none()
    {
        return Ok(Action::await_change());
    }

    info!("reconcile request: {}", obj.name_any());
    // We will only be supporting 1 exit node for all services for now, so we can just grab the first one

    let client = Client::try_default().await.unwrap();
    let nodes: Api<ExitNode> = Api::all(client.clone());
    let lp = ListParams::default().timeout(30);
    let services: Api<Service> = Api::namespaced(client.clone(), &obj.namespace().unwrap());
    let node_list = nodes.list(&lp).await.unwrap();
    let node = node_list.items.first();

    tracing::debug!("node: {:?}", node);

    let deployments: Api<Deployment> = Api::namespaced(client, &node.unwrap().namespace().unwrap());

    // TODO: make it DRY

    let ingress = match node.map(|n| n.spec.host.clone()) {
        Some(host) => serde_json::json!([{ "ip": host }]),
        None => serde_json::json!([]),
    };
    let external_ip = match node.map(|n| n.spec.host.clone()) {
        Some(host) => serde_json::json!([host]),
        None => serde_json::json!([]),
    };

    let spec_patch = serde_json::json!({"spec": {
        "loadBalancerIP": node.map(|n| n.spec.host.clone()),
        "externalIPs": external_ip,
    }
    });
    // set spec params

    let service = services
        .patch(
            obj.meta().name.as_ref().unwrap(),
            &PatchParams::default(),
            &Patch::Merge(spec_patch),
        )
        .await
        .unwrap();

    let deployment_data = create_owned_deployment(&service, &node.unwrap());
    let serverside = PatchParams::apply("chisel-operator");
    let _deployment = deployments
        .patch(
            &deployment_data.name_any(),
            &serverside,
            &Patch::Apply(deployment_data),
        )
        .await
        .unwrap();

    // set status
    let status_data = serde_json::json!({"status": {
        "message": "Chisel LoadBalancer Reconciled successfully",
        "loadBalancer": {
            "ingress": ingress
        }
    }});
    info!(status = ?status_data, "Patched status for {}", obj.name_any());

    services
        .patch_status(
            obj.meta().name.as_ref().unwrap(),
            &PatchParams::default(),
            &Patch::Merge(status_data),
        )
        .await
        .unwrap();

    Ok(Action::requeue(Duration::from_secs(3600)))
}

fn error_policy(_object: Arc<Service>, _err: &ReconcileError, _ctx: Arc<()>) -> Action {
    Action::requeue(Duration::from_secs(5))
}

pub async fn run() -> color_eyre::Result<()> {
    let client = Client::try_default().await.unwrap();
    // watch for K8s service resources (default)
    let services: Api<Service> = Api::all(client);

    Controller::new(services, Config::default())
        .run(reconcile, error_policy, Arc::new(()))
        .for_each(|_| futures::future::ready(()))
        .await;

    Ok(())
}
