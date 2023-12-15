use super::{cloud_init::generate_cloud_init_config, pwgen::generate_password, Provisioner};
use crate::{
    cloud::CHISEL_PORT,
    ops::{ExitNode, ExitNodeStatus, EXIT_NODE_PROVISIONER_LABEL},
};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use base64::Engine;
use color_eyre::eyre::{anyhow, Error};
use k8s_openapi::api::core::v1::Secret;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
const AMI_ID: &str = "ami-0df4b2961410d4cff";

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct AWSProvisioner {
    /// Region ID for the AWS region to provision the exit node in
    pub region: String,
    /// Reference to a secret containing the AWS access key ID and secret access key, under the access_key_id and secret_access_key keys
    pub auth: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AWSIdentity {
    access_key_id: String,
    secret_access_key: String,
    pub region: String,
}

impl AWSIdentity {
    pub fn new(access_key_id: String, secret_access_key: String, region: String) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            region,
        }
    }
    /// Generate an AWS config from the access key ID and secret access key
    pub async fn generate_aws_config(self) -> color_eyre::Result<aws_config::SdkConfig> {
        // set access key id and secret access key as environment variables
        std::env::set_var("AWS_ACCESS_KEY_ID", &self.access_key_id);
        std::env::set_var("AWS_SECRET_ACCESS_KEY", &self.secret_access_key);
        let region: String = self.region.clone();
        Ok(aws_config::defaults(BehaviorVersion::latest())
            .region(&*region.leak())
            .load()
            .await)
    }

    // This code is very unholy, but thanks to Jeff Bezos for making the AWS SDK so complicated
    pub fn from_secret(secret: &Secret, region: String) -> color_eyre::Result<Self> {
        let aws_access_key_id: String = secret
            .data
            .as_ref()
            .and_then(|f| f.get("AWS_ACCESS_KEY_ID"))
            .ok_or_else(|| anyhow!("AWS_ACCESS_KEY_ID not found in secret"))
            // into String
            .and_then(|key| String::from_utf8(key.clone().0.to_vec()).map_err(|e| e.into()))?;

        let aws_secret_access_key: String = secret
            .data
            .as_ref()
            .and_then(|f| f.get("AWS_SECRET_ACCESS_KEY"))
            .ok_or_else(|| anyhow!("AWS_SECRET_ACCESS_KEY not found in secret"))
            .and_then(|key| String::from_utf8(key.clone().0.to_vec()).map_err(|e| e.into()))?;

        Ok(Self {
            access_key_id: aws_access_key_id,
            secret_access_key: aws_secret_access_key,
            region,
        })
    }
}

#[async_trait]
impl Provisioner for AWSProvisioner {
    async fn create_exit_node(
        &self,
        auth: Secret,
        exit_node: ExitNode,
    ) -> color_eyre::Result<ExitNodeStatus> {
        let provisioner = exit_node
        .metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(EXIT_NODE_PROVISIONER_LABEL))
        .ok_or_else(|| {
            anyhow!(
                "No provisioner found in annotations for exit node {}",
                exit_node.metadata.name.as_ref().unwrap()
            )
        })?;

        let password = generate_password(32);

        let cloud_init_config = generate_cloud_init_config(&password, CHISEL_PORT);
        let user_data = base64::engine::general_purpose::STANDARD.encode(cloud_init_config);

        let aws_api = AWSIdentity::from_secret(&auth, self.region.clone())?
            .generate_aws_config()
            .await?;

        let ec2_client = aws_sdk_ec2::Client::new(&aws_api);

        let tag = format!("chisel-operator-provisioner:{}", provisioner);

        ec2_client.run_instances()
            .image_id(AMI_ID)
            .instance_type("t2.micro".into())
            .user_data(&user_data)
            .send();

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
