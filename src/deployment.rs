//! Chisel pod deployment

use crate::{
    error::ReconcileError,
    ops::{ExitNode, EXIT_NODE_PROXY_PROTOCOL_ANNOTATION},
};
use color_eyre::Result;
use k8s_openapi::{
    api::{
        apps::v1::{Deployment, DeploymentSpec},
        core::v1::{
            Container, EnvVar, EnvVarSource, PodSpec, PodTemplateSpec, SecretKeySelector, Service,
            ServicePort,
        },
    },
    apimachinery::pkg::apis::meta::v1::LabelSelector,
};
use kube::{core::ObjectMeta, error::ErrorResponse, Resource};
use tracing::{info, instrument, trace};

const CHISEL_IMAGE: &str = "jpillora/chisel";

fn get_protocol_suffix(svcport: &ServicePort) -> &'static str {
    svcport
        .protocol
        .as_ref()
        .map(|p| match p.as_str() {
            "TCP" => "/tcp",
            "UDP" => "/udp",
            _ => "",
        })
        .unwrap_or("")
}

/// This function generates a remote argument string using an ExitNode's host and port information.
///
/// Arguments:
///
/// * `node`: `node` is a reference to an `ExitNode` struct, which contains information about a specific
///   exit node in a network. The function `generate_remote_arg` takes this node as input and generates a
///   remote argument that can be used to connect to the exit node.
///
/// Returns:
///
/// The function `generate_remote_arg` is returning a `String`. The `String`
/// contains the formatted remote argument which is a combination of the `lb_ip` and `chisel_port`
/// values obtained from the `node` parameter.
use std::net::IpAddr;

pub fn generate_remote_arg(node: &ExitNode) -> String {
    // todo: what about ECDSA keys?

    let host = node.get_host();

    trace!(host = ?host, "Host");

    // Determine if the host is an IPv6 address and format accordingly
    let formatted_host = match host.parse::<IpAddr>() {
        Ok(IpAddr::V6(_)) => format!("[{host}]"),
        _ => host.to_string(),
    };

    let output = format!("{formatted_host}:{}", node.spec.port);
    trace!(output = ?output, "Output");
    output
}

/// This function generates arguments for a tunnel based on a given service.
///
/// Arguments:
///
/// * `svc`: `svc` is a reference to a `Service` object, which represents a set of pods that provide a
///   common network service. The function `generate_tunnel_args` takes this `Service` object as input and
///   generates a set of arguments that can be used to create a tunnel to the service.
///
/// Returns:
///
/// a `Result` containing a `Vec` of `String`s. The `Vec` contains arguments for a tunnel, which are
/// generated based on the input `Service`.
pub fn generate_tunnel_args(svc: &Service) -> Result<Vec<String>, ReconcileError> {
    let proxy_protocol = svc.metadata.annotations.as_ref().and_then(|annotations| {
        annotations
            .get(EXIT_NODE_PROXY_PROTOCOL_ANNOTATION)
            .map(String::as_ref)
    }) == Some("true");
    let target_ip = if proxy_protocol { "RP" } else { "R" };

    // Use ClusterIP directly instead of DNS name for more reliable routing
    let cluster_ip = svc
        .spec
        .as_ref()
        .and_then(|spec| spec.cluster_ip.as_ref())
        .ok_or(ReconcileError::NoClusterIP)?;

    // We can unwrap safely since Service is guaranteed to have a spec
    let ports = svc
        .spec
        .as_ref()
        .unwrap()
        .ports
        .as_ref()
        .ok_or(ReconcileError::NoPortsSet)?
        .iter()
        .map(|p| {
            // service_port = what the Service/ClusterIP listens on
            // (targetPort is only used internally by k8s to forward to pods)
            // Chisel connects to ClusterIP:service_port, k8s handles the rest

            // NOTE: Reverted from targetPort to using port directly to avoid confusion.
            // Turns out targetPort is meant for accessing the pods, not the Service itself.

            // If anyone knows the specifics of how CNIs actually handle this, please enlighten me.
            let service_port = p.port;
            let protocol = get_protocol_suffix(p);
            format!("{target_ip}:{service_port}:{cluster_ip}:{service_port}{protocol}")
        })
        .collect();

    info!("Generated arguments: {:?}", ports);
    trace!(svc = ?svc, "Source service");
    Ok(ports)
}

/// This function generates Chisel flags using various options set on an ExitNode's spec.
///
/// Arguments:
///
/// * `node`: `node` is a reference to an `ExitNode` struct, which contains information about a specific
/// exit node in a network. The function `generate_remote_arg` takes this node as input and generates a
/// Chisel flags that are used when connecting to the exit node.
///
/// Returns:
///
/// The function `generate_chisel_flags` is returning `Vec` of `String`s.
/// The `Vec` contains chisel flags for the client, which are
/// generated based on the input `ExitNode`'s spec.
#[instrument]
pub fn generate_chisel_flags(node: &ExitNode) -> Vec<String> {
    let mut flags = vec!["-v".to_string()];

    if let Some(fingerprint) = node.spec.fingerprint.to_owned() {
        flags.push("--fingerprint".to_string());
        flags.push(fingerprint)
    }

    flags
}

/// This function creates a PodTemplateSpec for a chisel container to be used as a tunnel between a
/// source service and an exit node.
///
/// Arguments:
///
/// * `source`: The `source` parameter is a reference to a `Service` object, which represents a set of
/// pods that provide a single, stable network endpoint for accessing a Kubernetes service.
/// * `exit_node`: `exit_node` is a reference to an `ExitNode` struct, which contains information about
/// the exit node that the pod will connect to. This includes the exit node's IP address, port, and
/// authentication credentials. The `generate_remote_arg` function is used to generate the command line
/// argument that
///
/// Returns:
///
/// a `PodTemplateSpec` object.
#[instrument(skip(source, exit_node))]
pub async fn create_pod_template(
    source: &Service,
    exit_node: &ExitNode,
) -> Result<PodTemplateSpec, ReconcileError> {
    let service_name = source.metadata.name.as_ref().ok_or_else(|| {
        ReconcileError::KubeError(kube::Error::Api(ErrorResponse {
            code: 500,
            message: "Service is missing name".to_string(),
            reason: "MissingServiceName".to_string(),
            status: "Failure".to_string(),
        }))
    })?;

    let mut args = vec!["client".to_string()];
    args.extend(generate_chisel_flags(exit_node));
    args.push(generate_remote_arg(exit_node));
    args.extend(generate_tunnel_args(source)?.iter().map(|s| s.to_string()));

    let env = exit_node.spec.auth.clone().map(|secret_name| {
        vec![EnvVar {
            name: "AUTH".to_string(),
            value_from: Some(EnvVarSource {
                secret_key_ref: Some(SecretKeySelector {
                    name: secret_name,
                    key: "auth".to_string(),
                    optional: Some(false),
                }),
                ..Default::default()
            }),
            ..Default::default()
        }]
    });

    // Warn when auth is not set
    if env.is_none() {
        tracing::warn!("No auth secret set for exit node! Tunnel will not be secure! This is a security risk!!!");
    }

    Ok(PodTemplateSpec {
        metadata: Some(ObjectMeta {
            labels: Some([("tunnel".to_string(), service_name.to_owned())].into()),
            ..Default::default()
        }),
        spec: Some(PodSpec {
            containers: vec![Container {
                args: Some(args),
                image: Some(
                    exit_node
                        .spec
                        .chisel_image
                        .clone()
                        .unwrap_or_else(|| CHISEL_IMAGE.to_string()),
                ),
                name: "chisel".to_string(),
                env,
                ..Default::default()
            }],
            ..Default::default()
        }),
    })
}

/// The function creates a deployment object for a service and exit node in Rust programming language.
///
/// Arguments:
///
/// * `source`: The `source` parameter is a reference to a `Service` object, which represents a set of
/// pods that perform the same function and are exposed by a common IP address and port.
/// * `exit_node`: An ExitNode is a node in a network that allows traffic to exit the network and reach
/// external services. In this context, it is likely being used to specify the node that the deployment
/// will be running on.
///
/// Returns:
///
/// a `Deployment` object.
#[instrument(skip(source, exit_node))]
pub async fn create_owned_deployment(
    source: &Service,
    exit_node: &ExitNode,
) -> Result<Deployment, ReconcileError> {
    let oref = exit_node.controller_owner_ref(&()).ok_or_else(|| {
        ReconcileError::KubeError(kube::Error::Api(ErrorResponse {
            code: 500,
            message: "Service is missing owner reference".to_string(),
            reason: "MissingOwnerReference".to_string(),
            status: "Failure".to_string(),
        }))
    })?;

    // cross namespace owner reference is not allowed so we link to exit node as its owner

    let service_name = source.metadata.name.as_ref().ok_or_else(|| {
        ReconcileError::KubeError(kube::Error::Api(ErrorResponse {
            code: 500,
            message: "Service is missing name".to_string(),
            reason: "MissingServiceName".to_string(),
            status: "Failure".to_string(),
        }))
    })?;

    Ok(Deployment {
        metadata: ObjectMeta {
            name: Some(format!("chisel-{service_name}")),
            owner_references: Some(vec![oref]),
            // namespace: exit_node.metadata.namespace.clone(),
            ..Default::default()
        },
        spec: Some(DeploymentSpec {
            template: create_pod_template(source, exit_node).await?,
            selector: LabelSelector {
                match_labels: Some([("tunnel".to_string(), service_name.to_owned())].into()),
                ..Default::default()
            },
            ..Default::default()
        }),
        ..Default::default()
    })
}

// #[cfg(test)]
// mod tests {
//     use crate::ops::ExitNodeSpec;

//     use super::*;
//     use k8s_openapi::api::core::v1::Service;
//     use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;

//     // TODO: ExitNode is missing owner reference, test fails
//     // TODO: implement more tests
//     #[test]
//     fn test_create_owned_deployment() {
//         let service = Service {
//             metadata: ObjectMeta {
//                 name: Some("test-service".to_string()),
//                 ..Default::default()
//             },
//             ..Default::default()
//         };
//         let exit_node = ExitNode {
//             spec: ExitNodeSpec {
//                 host: "127.0.0.1".to_string(),
//                 external_host: None,
//                 port: 8080,
//                 auth: None,
//                 fingerprint: None,
//                 default_route: true,
//             },
//             metadata: ObjectMeta {
//                 owner_references: Some(vec![OwnerReference {
//                     kind: "ExitNode".to_string(),
//                     api_version: "v1".to_string(),
//                     name: "test-node".to_string(),
//                     uid: uuid::Uuid::nil().to_string(),
//                     controller: Some(true),
//                     block_owner_deletion: Some(true),
//                 }]),
//                 namespace: Some("default".to_string()),
//                 ..Default::default()
//             },
//             status: None,
//         };
//         let deployment = create_owned_deployment(&service, &exit_node).await.unwrap();
//         assert_eq!(
//             deployment.metadata.name.unwrap(),
//             "chisel-test-service".to_string()
//         );
//         let owner_ref = deployment.metadata.owner_references.unwrap().pop().unwrap();
//         assert_eq!(owner_ref.kind, "ExitNode");
//         assert_eq!(owner_ref.api_version, "v1");
//         assert_eq!(owner_ref.name, "");
//         assert_eq!(owner_ref.uid, uuid::Uuid::nil().to_string());
//     }
// }
