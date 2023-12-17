// Daemon module
// watch for changes in all LoadBalancer services and update the IP addresses

/*
   notes:
   so the way this works is that the user deploys a ExitNodeProvisioner resource
   and then set an annotation on the service to use that provisioner
   the chisel operator will then watch for that annotation and then create a new exit node
   for that service
   the exit node will then be annotated with the name of the service
   if the service is deleted, the exit node will also be deleted, and the actual cloud resource will also be deleted

   honestly this whole logic is kinda confusing but I don't know how to make it less clunky


   There can also be a case where the user creates an exit node manually,
   with the provisioner annotation set, in that case chisel operator will
   create a cloud resource for that exit node and manages it.

   todo: properly handle all this logic

   todo: use `tracing` and put every operation in a span to make debugging easier
*/

use color_eyre::Result;
use futures::{FutureExt, StreamExt};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{LoadBalancerIngress, LoadBalancerStatus, Service, ServiceStatus},
};
use kube::{
    api::{Api, ListParams, Patch, PatchParams, ResourceExt},
    core::ObjectMeta,
    error::ErrorResponse,
    runtime::{
        controller::Action,
        finalizer::{self, Event},
        reflector::ObjectRef,
        watcher::{self, Config},
        Controller,
    },
    Client, Resource,
};
use std::{collections::BTreeMap, sync::Arc};

use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};

use crate::ops::{
    ExitNode, ExitNodeProvisioner, ExitNodeSpec, EXIT_NODE_NAME_LABEL, EXIT_NODE_PROVISIONER_LABEL,
};
use crate::{deployment::create_owned_deployment, error::ReconcileError};
#[allow(dead_code)]
pub const EXIT_NODE_FINALIZER: &str = "exitnode.chisel-operator.io/finalizer";
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

// this is actually used to pass clients around
#[allow(dead_code)]
pub struct Context {
    pub client: Client,
}

#[instrument(skip(ctx))]
async fn find_exit_node_from_label(ctx: Arc<Context>, query: &str) -> Option<ExitNode> {
    let nodes: Api<ExitNode> = Api::all(ctx.client.clone());
    let node_list = nodes.list(&ListParams::default().timeout(30)).await.ok()?;
    node_list.items.into_iter().find(|node| {
        node.metadata
            .name
            .as_ref()
            .map(|name| name == query)
            .unwrap_or(false)
    })
}
#[instrument(skip(ctx))]
async fn find_exit_node_provisioner_from_label(
    ctx: Arc<Context>,
    query: &str,
) -> Option<ExitNodeProvisioner> {
    let span = tracing::debug_span!("find_exit_node_provisioner_from_label", ?query);
    let _enter = span.enter();
    let nodes: Api<ExitNodeProvisioner> = Api::all(ctx.client.clone());
    let node_list = nodes.list(&ListParams::default().timeout(30)).await.ok()?;
    info!(node_list = ?node_list, "node list");
    let result = node_list.items.into_iter().find(|node| {
        node.metadata
            .name
            .as_ref()
            .map(|name| name == query)
            .unwrap_or(false)
    });
    debug!(query = ?query, ?result, "Query result");

    result
}
/// Check whether the exit node was managed by a provisioner
#[instrument]
async fn check_exit_node_managed(node: &ExitNode) -> bool {
    // returns false if there's no annotation, true if annotation exists, simple logic
    node.metadata
        .annotations
        .as_ref()
        .map(|annotations| annotations.contains_key(EXIT_NODE_PROVISIONER_LABEL))
        .unwrap_or(false)
}
#[instrument]
async fn check_service_managed(service: &Service) -> bool {
    // returns false if there's no annotation, true if annotation exists, simple logic
    service
        .metadata
        .annotations
        .as_ref()
        .map(|annotations| annotations.contains_key(EXIT_NODE_PROVISIONER_LABEL))
        .unwrap_or(false)
}

// Let's not use magic values, so we can change this later or if someone wants to fork this for something else

#[allow(dead_code)]
const OPERATOR_CLASS: &str = "chisel-operator.io/chisel-operator-class";
#[allow(dead_code)]
const OPERATOR_MANAGER: &str = "chisel-operator";
#[instrument(skip(ctx))]
async fn select_exit_node_local(
    ctx: Arc<Context>,
    service: &Service,
) -> Result<ExitNode, ReconcileError> {
    // if service has label with exit node name, use that and error if not found
    if let Some(exit_node_name) = service
        .metadata
        .labels
        .as_ref()
        .and_then(|labels| labels.get(EXIT_NODE_NAME_LABEL))
    {
        find_exit_node_from_label(ctx.clone(), exit_node_name)
            .await
            .ok_or(ReconcileError::NoAvailableExitNodes)
    } else {
        // otherwise, use the first available exit node
        let nodes: Api<ExitNode> = Api::all(ctx.client.clone());
        let node_list = nodes.list(&ListParams::default().timeout(30)).await?;
        node_list
            .items
            .into_iter()
            .filter(|node| {
                // Is the ExitNode not cloud provisioned OR is status set
                !node
                    .metadata
                    .annotations
                    .as_ref()
                    .map(|annotations| {
                        annotations.contains_key("chisel-operator.io/exit-node-provisioner")
                    })
                    .unwrap_or(false)
                    || node.status.is_some()
            })
            .collect::<Vec<ExitNode>>()
            .first()
            .ok_or(ReconcileError::NoAvailableExitNodes)
            .map(|node| node.clone())
    }
}
#[instrument(skip(ctx))]
async fn select_exit_node_cloud(
    ctx: Arc<Context>,
    service: &Service,
    provisioner: &str,
) -> Result<ExitNode, ReconcileError> {
    // logic is: it should check if the annotation is set, if it is not, create a new exit node and provision it
    // if it is set, then check if exit node exists, if it does, return that exit node, if it doesn't, create a new exit node and return that

    // check if annotation is set

    // if let Some(exit_node_name) = service
    //     .metadata
    //     .annotations
    //     .as_ref()
    //     .and_then(|annotations| annotations.get(EXIT_NODE_NAME_LABEL))
    // {
    //     let exit_node = find_exit_node_from_label(ctx, exit_node_name).await;
    //     return exit_node.ok_or(ReconcileError::NoAvailableExitNodes);
    // }

    // create new exit node here
    let node = exit_node_from_service(ctx, service);
    node.await
}

#[instrument(skip(ctx))]
/// Generates an ExitNode resource from a Service resource, and creates it
async fn exit_node_from_service(
    ctx: Arc<Context>,
    service: &Service,
) -> Result<ExitNode, ReconcileError> {
    let nodes: Api<ExitNode> = Api::namespaced(ctx.client.clone(), &service.namespace().unwrap());

    // check if annotation was set
    let provisioner_name = service
        .metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(EXIT_NODE_PROVISIONER_LABEL))
        .ok_or_else(|| ReconcileError::CloudProvisionerNotFound)?;

    let exit_node_name = service
        .metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(EXIT_NODE_NAME_LABEL))
        .unwrap_or({
            let service_name = service.metadata.name.as_ref().unwrap();
            &format!("service-{}", service_name)
        })
        .to_owned();

    let oref = service.controller_owner_ref(&()).ok_or_else(|| {
        ReconcileError::KubeError(kube::Error::Api(ErrorResponse {
            code: 500,
            message: "Service is missing owner reference".to_string(),
            reason: "MissingOwnerReference".to_string(),
            status: "Failure".to_string(),
        }))
    })?;

    // try to find exit node from name and namespace

    let exit_node_tmpl = ExitNode {
        metadata: ObjectMeta {
            name: Some(exit_node_name.clone()),
            namespace: service.namespace(),
            annotations: Some({
                let mut map = BTreeMap::new();
                map.insert(
                    EXIT_NODE_PROVISIONER_LABEL.to_string(),
                    provisioner_name.to_string(),
                );
                map
            }),
            owner_references: Some(vec![oref]),
            ..Default::default()
        },
        spec: ExitNodeSpec {
            host: "".to_string(),
            port: crate::cloud::CHISEL_PORT,
            auth: None,
            external_host: None,
            default_route: true,
            fingerprint: None,
        },
        status: None,
    };

    let exit_node = nodes.get(&exit_node_name).await;

    if let Ok(exit_node) = exit_node {
        return Ok(exit_node);
    } else {
        let serverside = PatchParams::apply(OPERATOR_MANAGER).validation_strict();

        let exit_node = nodes
            .patch(
                &exit_node_tmpl.name_any(),
                &serverside,
                &Patch::Apply(exit_node_tmpl.clone()),
            )
            .await?;

        Ok(exit_node)
    }
}
// #[instrument(skip(ctx), fields(trace_id))]
/// Reconcile cluster state
#[instrument(skip(ctx))]
async fn reconcile_svcs(obj: Arc<Service>, ctx: Arc<Context>) -> Result<Action, ReconcileError> {
    // Return if service is not LoadBalancer or if the loadBalancerClass is not blank or set to $OPERATOR_CLASS

    // todo: is there anything different need to be done for OpenShift? We use vanilla k8s and k3s/rke2 so we don't know
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
                    || spec.load_balancer_class == Some(OPERATOR_CLASS.to_string())
            })
            .is_none()
    {
        return Ok(Action::await_change());
    }

    info!("reconcile request: {}", obj.name_any());

    // We can unwrap safely since Service is namespaced scoped
    let services: Api<Service> = Api::namespaced(ctx.client.clone(), &obj.namespace().unwrap());

    let mut svc = services.get_status(&obj.name_any()).await?;

    let obj = svc.clone();

    let node = {
        if check_service_managed(&obj).await {
            let provisioner = obj
                .metadata
                .annotations
                .as_ref()
                .and_then(|annotations| annotations.get(EXIT_NODE_PROVISIONER_LABEL))
                .unwrap();

            // Remove attached exit node if the service was managed by a cloud provider and when it is removed
            let mut exit_node = select_exit_node_cloud(ctx.clone(), &obj, provisioner).await?;

            while exit_node.status.is_none() {
                warn!("Waiting for exit node to be provisioned");
                tokio::time::sleep(Duration::from_secs(5)).await;
                exit_node = select_exit_node_cloud(ctx.clone(), &obj, provisioner).await?;
            }

            exit_node
        } else {
            select_exit_node_local(ctx.clone(), &obj).await?
        }
    };

    let exit_node_ip = node.get_host();

    // check if status is the same as the one we're about to patch

    let obj_ip = obj.clone().status;

    debug!(?exit_node_ip, ?obj_ip, "Exit node IP debug");

    let serverside = PatchParams::apply(OPERATOR_MANAGER).validation_strict();

    // let mut svc = obj.clone();

    // debug!(?exit_node_ip, "Exit node IP");

    if svc
        .status
        .as_ref()
        .and_then(|status| status.load_balancer.as_ref())
        .and_then(|lb| lb.ingress.as_ref())
        .and_then(|ingress| ingress.first())
        .and_then(|ingress| ingress.ip.as_ref())
        == Some(&exit_node_ip)
    {
        info!("Load balancer IP is already set, not patching");
        return Ok(Action::requeue(Duration::from_secs(3600)));
    }

    svc.status = Some(ServiceStatus {
        load_balancer: Some(LoadBalancerStatus {
            ingress: Some(vec![LoadBalancerIngress {
                ip: Some(exit_node_ip),
                // hostname: Some(node.get_external_host()),
                ..Default::default()
            }]),
        }),
        ..Default::default()
    });

    // Update the status for the LoadBalancer service
    // The ExitNode IP will always be set, so it is safe to unwrap the host

    debug!(status = ? svc.status, "Service status");

    // debug!("Patching status for {}", obj.name_any());

    let _svcs = services
        .patch_status(
            // We can unwrap safely since Service is guaranteed to have a name
            obj.name_any().as_str(),
            &serverside.clone(),
            &Patch::Merge(&svc),
        )
        .await?;

    info!(status = ?obj, "Patched status for {}", obj.name_any());

    // We can unwrap safely since ExitNode is namespaced scoped
    let deployments: Api<Deployment> =
        Api::namespaced(ctx.client.clone(), &node.namespace().unwrap());

    // TODO: We should refactor this such that each deployment of Chisel corresponds to an exit node
    // Currently each deployment of Chisel corresponds to a service, which means duplicate deployments of Chisel
    // This also caused some issues, where we (intuitively) made the owner ref of the deployment the service
    // which breaks since a service can be in a seperate namespace from the deployment (k8s disallows this)
    let deployment_data = create_owned_deployment(&obj, &node).await?;
    let _deployment = deployments
        .patch(
            &deployment_data.name_any(),
            &serverside,
            &Patch::Apply(deployment_data),
        )
        .await?;

    tracing::trace!("deployment: {:?}", _deployment);

    Ok(Action::requeue(Duration::from_secs(3600)))

    // if obj_ip == Some(exit_node_ip.as_str())
    // {
    //     info!("status is the same, not patching");

    //     return Ok(Action::requeue(Duration::from_secs(3600)));
    // } else {
    // }
}

#[instrument(skip(_object, err, _ctx))]
fn error_policy(_object: Arc<Service>, err: &ReconcileError, _ctx: Arc<Context>) -> Action {
    error!(err = ?err);
    Action::requeue(Duration::from_secs(5))
}

#[instrument(skip(_object, err, _ctx))]
fn error_policy_exit_node(
    _object: Arc<ExitNode>,
    err: &ReconcileError,
    _ctx: Arc<Context>,
) -> Action {
    error!(err = ?err);
    Action::requeue(Duration::from_secs(5))
}

#[instrument(skip(ctx))]
async fn reconcile_nodes(obj: Arc<ExitNode>, ctx: Arc<Context>) -> Result<Action, ReconcileError> {
    info!("exit node reconcile request: {}", obj.name_any());

    let is_managed = check_exit_node_managed(&obj).await;

    debug!(?is_managed, "exit node is managed by cloud provisioner?");

    // if !is_managed {
    //     return Ok(Action::await_change());
    // }

    let provisioner = obj
        .metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(EXIT_NODE_PROVISIONER_LABEL))
        .unwrap();

    let provisioner = find_exit_node_provisioner_from_label(ctx.clone(), provisioner)
        .await
        .ok_or(ReconcileError::CloudProvisionerNotFound)?;

    let exit_nodes: Api<ExitNode> = Api::namespaced(ctx.client.clone(), &obj.namespace().unwrap());

    let mut exitnode_patchtmpl = exit_nodes.get(&obj.name_any()).await?;

    let provisioner_api = provisioner.clone().spec.get_inner();

    // finalizer for exit node
    let secret = provisioner
        .find_secret()
        .await
        .or_else(|_| Err(crate::error::ReconcileError::CloudProvisionerSecretNotFound))?
        .ok_or(ReconcileError::CloudProvisionerSecretNotFound)?;

    let serverside = PatchParams::apply(OPERATOR_MANAGER).validation_strict();

    //? maybe we move this to finalizer's apply function?

    let _node = {
        let cloud_resource = if let Some(_status) = obj.status.as_ref() {
            info!("Updating cloud resource for {}", obj.name_any());
            provisioner_api
                .update_exit_node(secret.clone(), (*obj).clone())
                .await
        } else {
            info!("Creating cloud resource for {}", obj.name_any());
            provisioner_api
                .create_exit_node(secret.clone(), (*obj).clone())
                .await
        };
        exitnode_patchtmpl.status = Some(cloud_resource?);

        exit_nodes
            .patch_status(
                // We can unwrap safely since Service is guaranteed to have a name
                &obj.name_any(),
                &serverside.clone(),
                &Patch::Merge(exitnode_patchtmpl),
            )
            .await?
    };

    // handle deletion
    finalizer::finalizer(
        &exit_nodes,
        EXIT_NODE_FINALIZER,
        obj.clone(),
        |event| async move {
            let m: std::prelude::v1::Result<Action, crate::error::ReconcileError> = match event {
                Event::Apply(_) => Ok(Action::requeue(Duration::from_secs(3600))),
                Event::Cleanup(node) => {
                    info!("Cleanup finalizer triggered for {}", node.name_any());

                    if is_managed {
                        info!("Deleting cloud resource for {}", node.name_any());
                        provisioner_api
                            .delete_exit_node(secret, (*node).clone())
                            .await
                            .unwrap_or_else(|e| {
                                error!(?e, "Error deleting exit node {}", node.name_any())
                            });
                    }
                    Ok(Action::requeue(Duration::from_secs(3600)))
                }
            };

            // Ok(Action::requeue(Duration::from_secs(3600)))
            m
        },
    )
    .await
    .map_err(|e| {
        crate::error::ReconcileError::KubeError(kube::Error::Api(kube::error::ErrorResponse {
            code: 500,
            message: format!("Error applying finalizer for {}", obj.name_any()),
            reason: e.to_string(),
            status: "Failure".to_string(),
        }))
    })

    // Ok(Action::requeue(Duration::from_secs(3600)))
}

/// watches for Kubernetes service resources and runs a controller to reconcile them.
#[instrument]
pub async fn run() -> color_eyre::Result<()> {
    let client = Client::try_default().await?;
    // watch for K8s service resources (default)
    let services: Api<Service> = Api::all(client.clone());

    let exit_nodes: Api<ExitNode> = Api::all(client.clone());

    let mut reconcilers = vec![];

    info!("Starting reconcilers...");

    reconcilers.push(
        Controller::new(services, Config::default())
            .watches(
                Api::<ExitNode>::all(client.clone()),
                watcher::Config::default(),
                |node: ExitNode| {
                    node.metadata
                        .annotations
                        .as_ref()
                        .unwrap_or(&BTreeMap::new())
                        .get(EXIT_NODE_PROVISIONER_LABEL)
                        .map(String::as_str)
                        .map(ObjectRef::new)
                },
            )
            .run(
                reconcile_svcs,
                error_policy,
                Arc::new(Context {
                    client: client.clone(),
                }),
            )
            .for_each(|_| futures::future::ready(()))
            .boxed(),
    );

    // I actually don't know from which way the watcher goes, so I'm just gonna put it here
    reconcilers.push(
        Controller::new(exit_nodes, Config::default())
            .watches(
                Api::<Service>::all(client.clone()),
                watcher::Config::default(),
                |node: Service| {
                    node.metadata
                        .annotations
                        .as_ref()
                        .unwrap_or(&BTreeMap::new())
                        .get(EXIT_NODE_PROVISIONER_LABEL)
                        .map(String::as_str)
                        .map(ObjectRef::new)
                },
            )
            .run(
                reconcile_nodes,
                error_policy_exit_node,
                Arc::new(Context { client }),
            )
            .for_each(|_| futures::future::ready(()))
            .boxed(),
    );

    futures::future::join_all(reconcilers).await;

    Ok(())
}
