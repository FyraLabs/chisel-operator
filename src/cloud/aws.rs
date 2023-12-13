use super::{cloud_init::generate_cloud_init_config, pwgen::generate_password, Provisioner};
use crate::ops::{ExitNode, ExitNodeStatus, EXIT_NODE_PROVISIONER_LABEL};
use async_trait::async_trait;
use color_eyre::eyre::{anyhow, Error};
use k8s_openapi::api::core::v1::Secret;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct AWSProvisioner {
    /// Region ID of the DigitalOcean datacenter to provision the exit node in
    /// If empty, DigitalOcean will randomly select a region for you, which might not be what you want
    #[serde(default)]
    pub region: String,
    /// Reference to a secret containing the DigitalOcean API token, under the token key
    pub auth: String,

}

#[async_trait]
impl Provisioner for AWSProvisioner {
    async fn create_exit_node(
        &self,
        auth: Secret,
        exit_node: ExitNode,
    ) -> color_eyre::Result<ExitNodeStatus> {
        unimplemented!()
    }

    async fn update_exit_node(
        &self,
        auth: Secret,
        exit_node: ExitNode,
    ) -> color_eyre::Result<ExitNodeStatus> {
        unimplemented!()
    }

    async fn delete_exit_node(&self, auth: Secret, exit_node: ExitNode) -> color_eyre::Result<()> {
        unimplemented!()
    }

}
