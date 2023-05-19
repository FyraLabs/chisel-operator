//! Chisel pod deployment

use crate::ops::ExitNode;
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
use kube::{api::ResourceExt, core::ObjectMeta, Resource};
use tracing::{debug, info};

fn convert_service_port(svcport: ServicePort) -> String {
    let mut port = String::new();

    // get port number
    port.push_str(&svcport.port.to_string());

    if let Some(protocol) = svcport.protocol {
        match protocol.as_str() {
            "TCP" => port.push_str("/tcp"),
            "UDP" => port.push_str("/udp"),
            _ => (),
        };
    }

    port
}

pub fn generate_remote_arg(node: &ExitNode) -> Result<String> {
    // get chisel host from config
    let lb_ip = &node.spec.host;

    // get chisel port from config
    let chisel_port = node.spec.port;

    Ok(format!("{}:{}", lb_ip, chisel_port))
}

pub fn generate_tunnel_args(svc: &Service) -> Result<Vec<String>> {
    let service_name = svc.metadata.name.clone().unwrap();
    let service_namespace = svc.namespace().unwrap();

    // check if there's a custom IP set
    // let target_ip = svc
    //     .spec
    //     .as_ref()
    //     .map(|spec| spec.load_balancer_ip.clone())
    //     .flatten()
    //     .unwrap_or_else(|| "R".to_string());

    let target_ip = "R";

    let ports = svc
        .spec
        .as_ref()
        .unwrap()
        .ports
        .as_ref()
        .unwrap()
        .iter()
        .map(|p| {
            format!(
                "{}:{}:{}:{}",
                target_ip,
                p.port.to_string(),
                format!("{}.{}", service_name, service_namespace),
                convert_service_port(p.clone())
            )
        }).collect();

    info!("Generated arguments: {:?}", ports);
    debug!(svc = ?svc, "Source service");
    Ok(ports)
}

pub fn create_pod_template(source: &Service, exit_node: &ExitNode) -> PodTemplateSpec {
    let mut args = vec![
        "client".to_string(),
        "-v".to_string(),
        generate_remote_arg(exit_node).unwrap(),
    ];
    args.extend(
        generate_tunnel_args(source)
            .unwrap()
            .iter()
            .map(|s| s.to_string()),
    );

    let env = exit_node.spec.auth.clone().map(|secret_name| {
        vec![EnvVar {
            name: "AUTH".to_string(),
            value_from: Some(EnvVarSource {
                secret_key_ref: Some(SecretKeySelector {
                    name: Some(secret_name),
                    key: "auth".to_string(),
                    optional: Some(false),
                }),
                ..Default::default()
            }),
            ..Default::default()
        }]
    });

    PodTemplateSpec {
        metadata: Some(ObjectMeta {
            labels: Some([("tunnel".to_string(), source.metadata.name.clone().unwrap())].into()),
            ..Default::default()
        }),
        spec: Some(PodSpec {
            containers: vec![Container {
                args: Some(args),
                image: Some("jpillora/chisel".to_string()),
                name: "chisel".to_string(),
                env,
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub fn create_owned_deployment(source: &Service, exit_node: &ExitNode) -> Deployment {
    let oref = source.controller_owner_ref(&()).unwrap();

    Deployment {
        metadata: ObjectMeta {
            name: Some(format!("chisel-{}", source.metadata.name.clone().unwrap())),
            owner_references: Some(vec![oref]),
            ..ObjectMeta::default()
        },
        spec: Some(DeploymentSpec {
            template: create_pod_template(source, exit_node),
            selector: LabelSelector {
                match_labels: Some(
                    [("tunnel".to_string(), source.metadata.name.clone().unwrap())].into(),
                ),
                ..Default::default()
            },
            ..Default::default()
        }),
        ..Default::default()
    }
}
