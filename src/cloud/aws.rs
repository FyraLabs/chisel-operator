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
    /// Region ID for the AWS region to provision the exit node in
    pub region: String,
    /// Reference to a secret containing the AWS access key ID and secret access key, under the access_key_id and secret_access_key keys
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
