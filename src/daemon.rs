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

use std::env;

use crate::{
    cloud::Provisioner,
    ops::{
        parse_provisioner_label_value, ExitNode, ExitNodeProvisioner, ExitNodeSpec, ExitNodeStatus,
        EXIT_NODE_NAME_LABEL, EXIT_NODE_PROVISIONER_LABEL,
    },
};
use crate::{deployment::create_owned_deployment, error::ReconcileError};

pub const EXIT_NODE_FINALIZER: &str = "exitnode.chisel-operator.io/finalizer";
pub const SVCS_FINALIZER: &str = "service.chisel-operator.io/finalizer";

// todo: Refactor everything in here into separate functions, then we can write unit tests for them

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
pub struct Context {
    pub client: Client,
}

/// Parses the `query` string to extract the namespace and name.
/// If the `query` contains a '/', it splits the `query` into two parts:
/// the namespace and the name. Otherwise, it uses the `og_namespace`
/// as the namespace and the entire `query` as the name.
///
/// # Arguments
///
/// * `query` - A string slice that holds the query to be parsed.
/// * `og_namespace` - A string slice that holds the original namespace.
///
/// # Returns
///
/// A tuple containing the namespace and name as string slices.
#[instrument(skip(ctx))]
async fn find_exit_node_from_label(
    ctx: Arc<Context>,
    query: &str,
    og_namespace: &str,
) -> Option<ExitNode> {
    // parse the query to get the namespace and name
    let (namespace, name) = if let Some((ns, nm)) = query.split_once('/') {
        (ns, nm)
    } else {
        // if the query does not contain a '/', use the original namespace
        (og_namespace, query)
    };

    let nodes: Api<ExitNode> = Api::namespaced(ctx.client.clone(), namespace);
    let node_list = nodes.list(&ListParams::default().timeout(30)).await.ok()?;
    node_list.items.into_iter().find(|node| {
        node.metadata
            .name
            .as_ref()
            .map(|n| n == name)
            .unwrap_or(false)
    })
}

#[instrument(skip(ctx))]
async fn find_exit_node_provisioner_from_label(
    ctx: Arc<Context>,
    default_namespace: &str,
    query: &str,
) -> Option<ExitNodeProvisioner> {
    let span = tracing::debug_span!("find_exit_node_provisioner_from_label", ?query);
    let _enter = span.enter();

    let (namespace, name) = parse_provisioner_label_value(default_namespace, query);

    let nodes: Api<ExitNodeProvisioner> = Api::namespaced(ctx.client.clone(), namespace);
    let node_list = nodes.list(&ListParams::default().timeout(30)).await.ok()?;
    info!(node_list = ?node_list, "node list");
    let result = node_list.items.into_iter().find(|node| {
        node.metadata
            .name
            .as_ref()
            .map(|n| n == name)
            .unwrap_or(false)
    });
    debug!(name = ?name, ?result, "Query result");

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

const OPERATOR_CLASS: &str = "chisel-operator.io/chisel-operator-class";
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
        info!(
            ?exit_node_name,
            "Service explicitly set to use a named exit node, using that"
        );
        find_exit_node_from_label(
            ctx.clone(),
            exit_node_name,
            &service.namespace().expect("Service namespace not found"),
        )
        .await
        .ok_or(ReconcileError::NoAvailableExitNodes)
    } else {
        // otherwise, use the first available exit node
        // (one to one mapping)
        let nodes: Api<ExitNode> = Api::all(ctx.client.clone());
        let node_list: kube::core::ObjectList<ExitNode> =
            nodes.list(&ListParams::default().timeout(30)).await?;
        node_list
            .items
            .into_iter()
            .filter(|node| {
                let is_cloud_provisioned = node
                    .metadata
                    .annotations
                    .as_ref()
                    .map(|annotations: &BTreeMap<String, String>| {
                        annotations.contains_key(EXIT_NODE_PROVISIONER_LABEL)
                    })
                    .unwrap_or(false);

                // Is the ExitNode not cloud provisioned or is the status set?
                (!is_cloud_provisioned || node.status.is_some()) && !node.is_assigned()
            })
            .collect::<Vec<ExitNode>>()
            .first()
            .ok_or(ReconcileError::NoAvailableExitNodes)
            .cloned()
    }
}

#[instrument(skip(ctx))]
/// Returns the ExitNode resource for a Service resource, either finding an existing one or creating a new one
async fn exit_node_for_service(
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

    // try to find exit node from name within the service's namespace, and return if found
    if let Ok(exit_node) = nodes.get(&exit_node_name).await {
        return Ok(exit_node);
    }

    let exit_node_tmpl = ExitNode {
        metadata: ObjectMeta {
            name: Some(exit_node_name.clone()),
            namespace: service.namespace(),
            annotations: Some({
                let mut map = BTreeMap::new();
                map.insert(
                    EXIT_NODE_PROVISIONER_LABEL.to_string(),
                    format!("{}/{}", service.namespace().unwrap(), provisioner_name), // Fixes #38
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
            chisel_image: None,
        },
        status: None,
    };

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
// #[instrument(skip(ctx), fields(trace_id))]
/// Reconcile cluster state
#[instrument(skip(ctx))]
async fn reconcile_svcs(obj: Arc<Service>, ctx: Arc<Context>) -> Result<Action, ReconcileError> {
    // Return if service is not LoadBalancer or if the loadBalancerClass is not blank or set to $OPERATOR_CLASS

    // Check if the REQUIRE_OPERATOR_CLASS environment variable is set
    let limit_load_balancer_class;
    match env::var("REQUIRE_OPERATOR_CLASS") {
        Ok(v) => limit_load_balancer_class = v,
        Err(_e) => limit_load_balancer_class = "false".to_string(),
    }

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
                (spec.load_balancer_class.is_none() && ( limit_load_balancer_class == "false"))
                    || spec.load_balancer_class == Some(OPERATOR_CLASS.to_string())
            })
            .is_none()
    {
        return Ok(Action::await_change());
    }

    info!("reconcile request: {}", obj.name_any());

    // We can unwrap safely since Service is namespaced scoped
    let services: Api<Service> = Api::namespaced(ctx.client.clone(), &obj.namespace().unwrap());
    let nodes: Api<ExitNode> = Api::all(ctx.client.clone());

    let mut svc = services.get_status(&obj.name_any()).await?;

    let obj = svc.clone();

    let node_list = nodes.list(&ListParams::default().timeout(30)).await?;
    // Find service binding of svc name/namespace?
    let existing_node = node_list.iter().find(|node| {
        node.metadata
            .annotations
            .as_ref()
            .map(|annotations| annotations.contains_key(EXIT_NODE_NAME_LABEL))
            .unwrap_or(false)
    });

    let node = {
        if let Some(node) = existing_node {
            node.clone()
        } else if check_service_managed(&obj).await {
            // Remove attached exit node if the service was managed by a cloud provider and when it is removed
            let mut exit_node = exit_node_for_service(ctx.clone(), &obj).await?;

            while exit_node.status.is_none() {
                warn!("Waiting for exit node to be provisioned");
                tokio::time::sleep(Duration::from_secs(5)).await;
                exit_node = exit_node_for_service(ctx.clone(), &obj).await?;
            }

            exit_node
        }
        // If a service *specifically* chooses a named exit node, use that one
        // Allows support for multiple services to use the same exit node

        // Else, use the first available exit node
        // Fails if there's no empty exit node available
        else {
            select_exit_node_local(ctx.clone(), &obj).await?
        }
    };

    let exit_node_ip = node.get_host();

    // check if status is the same as the one we're about to patch

    let obj_ip = obj.clone().status;

    debug!(?exit_node_ip, ?obj_ip, "Exit node IP");

    let serverside = PatchParams::apply(OPERATOR_MANAGER).validation_strict();

    svc.status = Some(ServiceStatus {
        load_balancer: Some(LoadBalancerStatus {
            ingress: Some(vec![LoadBalancerIngress {
                ip: Some(exit_node_ip.clone()),
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

    info!(status = ?obj, "Patched status for service {}", obj.name_any());

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
            &Patch::Apply(deployment_data.clone()),
        )
        .await?;

    tracing::trace!(?_deployment);

    finalizer::finalizer(
        &services,
        SVCS_FINALIZER,
        obj.clone().into(),
        |event| async move {
            let m: std::prelude::v1::Result<Action, crate::error::ReconcileError> = match event {
                Event::Apply(_svc) => {
                    info!(status = ?node, "Patched status for ExitNode {}", node.name_any());
                    Ok(Action::requeue(Duration::from_secs(3600)))
                }
                Event::Cleanup(svc) => {
                    info!("Cleanup finalizer triggered for {}", svc.name_any());

                    // Clean up deployment when service is deleted
                    let deployments: Api<Deployment> =
                        Api::namespaced(ctx.client.clone(), &node.namespace().unwrap());

                    info!("Deleting deployment for {}", svc.name_any());

                    let _deployment = deployments
                        .delete(deployment_data.name_any().as_str(), &Default::default())
                        .await?;
                    Ok(Action::requeue(Duration::from_secs(3600)))
                }
            };
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
const UNMANAGED_PROVISIONER: &str = "unmanaged";
#[instrument(skip(ctx))]
async fn reconcile_nodes(obj: Arc<ExitNode>, ctx: Arc<Context>) -> Result<Action, ReconcileError> {
    info!("exit node reconcile request: {}", obj.name_any());

    let is_managed = check_exit_node_managed(&obj).await;

    debug!(?is_managed, "exit node is managed by cloud provisioner?");

    let exit_nodes: Api<ExitNode> = Api::namespaced(ctx.client.clone(), &obj.namespace().unwrap());

    // finalizer for exit node

    let serverside = PatchParams::apply(OPERATOR_MANAGER).validation_strict();

    if !is_managed && obj.status.is_none() {
        // add status to exit node if it's not managed

        let nodes: Api<ExitNode> = Api::namespaced(ctx.client.clone(), &obj.namespace().unwrap());

        let mut exitnode_patchtmpl = nodes.get(&obj.name_any()).await?;

        // now we set the status, but the provisioner is unmanaged
        // so we just copy the IP from the exit node config to the status

        let exit_node_ip = obj.get_host();

        exitnode_patchtmpl.status = Some(ExitNodeStatus {
            provider: UNMANAGED_PROVISIONER.to_string(),
            name: obj.name_any(),
            ip: exit_node_ip,
            id: None,
        });

        let serverside = PatchParams::apply(OPERATOR_MANAGER).validation_strict();

        let _node = nodes
            .patch_status(
                // We can unwrap safely since Service is guaranteed to have a name
                &obj.name_any(),
                &serverside.clone(),
                &Patch::Merge(exitnode_patchtmpl),
            )
            .await?;

        return Ok(Action::await_change());
    } else if is_managed {
        let provisioner = obj
            .metadata
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.get(EXIT_NODE_PROVISIONER_LABEL))
            .unwrap();
        if let Some(status) = &obj.status {
            // Check for mismatch between annotation's provisioner and status' provisioner
            if &status.provider != provisioner {
                // Destroy cloud resource
                warn!("Cloud provisioner mismatch, destroying cloud resource found in status");

                let old_provider = status.provider.clone();

                let old_provisioner = find_exit_node_provisioner_from_label(
                    ctx.clone(),
                    &obj.namespace().unwrap(),
                    &old_provider,
                )
                .await
                .ok_or(ReconcileError::CloudProvisionerNotFound)?;

                let old_provisioner_api: Box<dyn Provisioner + Send + Sync> =
                    old_provisioner.clone().spec.get_inner();

                let secret = old_provisioner
                    .find_secret()
                    .await
                    .map_err(|_| crate::error::ReconcileError::CloudProvisionerSecretNotFound)?
                    .ok_or(ReconcileError::CloudProvisionerSecretNotFound)?;

                old_provisioner_api
                    .delete_exit_node(secret, (*obj).clone())
                    .await?;

                // Now blank out the status

                let nodes: Api<ExitNode> =
                    Api::namespaced(ctx.client.clone(), &obj.namespace().unwrap());

                let exitnode_patch = serde_json::json!({
                    "status": None::<ExitNodeStatus>
                });

                info!("Clearing status for exit node {}", obj.name_any());

                let _node = nodes
                    .patch_status(
                        // We can unwrap safely since Service is guaranteed to have a name
                        &obj.name_any(),
                        &serverside.clone(),
                        &Patch::Merge(exitnode_patch),
                    )
                    .await?;
            }
        }

        // Find provisioner
        let provisioner = find_exit_node_provisioner_from_label(
            ctx.clone(),
            &obj.namespace().unwrap(),
            provisioner,
        )
        .await
        .ok_or(ReconcileError::CloudProvisionerNotFound)?;

        // Get provisioner API handle
        let provisioner_api = provisioner.clone().spec.get_inner();

        let secret = provisioner
            .find_secret()
            .await
            .map_err(|_| crate::error::ReconcileError::CloudProvisionerSecretNotFound)?
            .ok_or(ReconcileError::CloudProvisionerSecretNotFound)?;

        debug!(?secret, "Secret");

        finalizer::finalizer(
            &exit_nodes.clone(),
            EXIT_NODE_FINALIZER,
            obj.clone(),
            |event| async move {
                let m: std::prelude::v1::Result<Action, crate::error::ReconcileError> = match event
                {
                    Event::Apply(node) => {
                        let _node = {
                            let mut pass_secret: Option<k8s_openapi::api::core::v1::Secret> = None;
                            // if status exists, update, else create
                            let cloud_resource = if let Some(_status) = node.status.as_ref() {
                                info!("Updating cloud resource for {}", node.name_any());
                                provisioner_api
                                    .update_exit_node(secret.clone(), (*node).clone())
                                    .await
                            } else {
                                // todo: probably update the Provisioner trait to accept a provisioner API handle or
                                // the provisioner API token *and* then a password secret
                                // Right now we have the create_exit_node method which returns the password secret alongside the status

                                // create cloud resource
                                info!("Creating cloud resource for {}", node.name_any());

                                let (resource, new_pass_secret) = provisioner_api
                                    .create_exit_node(secret.clone(), (*node).clone())
                                    .await?;
                                pass_secret = Some(new_pass_secret);
                                Ok(resource)
                            };
                            // TODO: Don't replace the entire status and object, sadly JSON is better here
                            let exitnode_patch = if let Some(p_secret) = pass_secret {
                                serde_json::json!({
                                    "status": cloud_resource?,
                                    "spec": {
                                        "auth": p_secret.name_any(),
                                    }
                                })
                            } else {
                                serde_json::json!({
                                    "status": cloud_resource?,
                                })
                            };

                            exit_nodes
                                .patch_status(
                                    // We can unwrap safely since Service is guaranteed to have a name
                                    &node.name_any(),
                                    &serverside.clone(),
                                    &Patch::Merge(exitnode_patch),
                                )
                                .await?
                        };

                        Ok(Action::requeue(Duration::from_secs(3600)))
                    }
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
    } else {
        Ok(Action::requeue(Duration::from_secs(3600)))
    }
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

    // TODO: figure out how to do this in a single controller because there is a potential race where the exit node reconciler runs at the same time as the service one
    // This is an issue because both of these functions patch the status of the exit node
    // or if we can figure out a way to atomically patch the status of the exit node, that could be fine too, since both ops are just updates anyways lmfao

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
