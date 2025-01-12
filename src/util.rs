use std::{collections::BTreeMap, sync::Arc};

use k8s_openapi::api::core::v1::Service;
use kube::{api::ListParams, Api};

use crate::{daemon::Context, ops::ExitNode};
use color_eyre::Result;

/// Fetch exit nodes from the Kubernetes API in a map keyed by IP address
/// This is useful for quickly looking up exit nodes by IP address
///
/// # Arguments
///
/// * `ctx` - A shared context object
/// * `namespace` - An optional namespace to filter exit nodes by. If None, all namespaces are looked up
pub async fn get_exit_nodes_by_ip(
    ctx: Arc<Context>,
    namespace: Option<&str>,
) -> Result<BTreeMap<String, ExitNode>> {
    let exit_node_api: Api<ExitNode> = {
        if let Some(namespace) = namespace {
            Api::namespaced(ctx.client.clone(), namespace)
        } else {
            Api::all(ctx.client.clone())
        }
    };
    Ok(exit_node_api
        .list(&ListParams::default().timeout(30))
        .await?
        .items
        .into_iter()
        .filter_map(|node| {
            let host = node.get_host();
            if let Some(_status) = &node.status {
                Some((host, node))
            } else {
                None
            }
        })
        .collect())
}

pub fn get_svc_lb_ip(svc: &Service) -> Option<String> {
    svc.status.as_ref().and_then(|status| {
        status
            .load_balancer
            .as_ref()
            .and_then(|lb| lb.ingress.as_ref())
            .and_then(|ingress| ingress.first())
            .and_then(|ingress| ingress.ip.as_ref())
            .cloned()
    })
}

pub async fn get_svc_bound_exit_node(ctx: Arc<Context>, svc: &Service) -> Result<Option<ExitNode>> {
    let exit_nodes = get_exit_nodes_by_ip(ctx, None).await?;
    let svc_lb_ip = get_svc_lb_ip(svc);
    Ok(svc_lb_ip.and_then(|ip| exit_nodes.get(&ip).cloned()))
}
