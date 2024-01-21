use crate::ops::ExitNode;
use crate::ops::ExitNodeStatus;
use async_trait::async_trait;
use k8s_openapi::api::core::v1::Secret;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod aws;
mod cloud_init;
pub mod digitalocean;
pub mod linode;
pub mod pwgen;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum CloudProvider {
    DigitalOcean,
    Linode,
    AWS,
}

// This code was actually used, weirdly enough
#[allow(dead_code)]
pub const CHISEL_PORT: u16 = 9090;

#[async_trait]
pub trait Provisioner {
    async fn create_exit_node(
        &self,
        auth: Secret,
        exit_node: ExitNode,
    ) -> color_eyre::Result<ExitNodeStatus>;
    async fn update_exit_node(
        &self,
        auth: Secret,
        exit_node: ExitNode,
    ) -> color_eyre::Result<ExitNodeStatus>;
    async fn delete_exit_node(&self, auth: Secret, exit_node: ExitNode) -> color_eyre::Result<()>;
}

// Each LB service binds to an exit node, which will be a many-to-one relationship
// An LB can annotate a specific exit node to bind to, or it can specify a provider to automatically provision a new exit node
// if no specific exit node is specified and a provider is not specified, then the first exit node returned by the K8S API will be used
// but if provider is specified, then a new exit node will be provisioned on that provider
// A provisioner can have many exit nodes that it manages
// each exit node can be manually managed or automatically managed by a provisioner
// you can request a new exit node from a provisioner by simply creating a LB service without specifying a specific exit node
// or you can create a new managed exit node

// Take LB1 which has annotation chisel-operator.io/cloud-provisioner: do
// Take LB2 which has annotation chisel-operator.io/cloud-provisioner: do ON A DIFFERENT PORT
// what if I want to use the same exit node for both LB1 and LB2?
// maybe we can introduce a new annotation chisel-operator.io/cloud-exit-node: <name>
// if two LBs have the same cloud-exit-node annotation, then they will use the same exit node, WHEN THE PROVISIONER IS THE SAME
