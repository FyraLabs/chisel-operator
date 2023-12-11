use super::{
    cloud_init::generate_cloud_init_config, pwgen::generate_password, CloudExitNode, Provisioner,
};
use crate::ops::{ExitNode, ExitNodeProvisioner, ExitNodeStatus};
use async_trait::async_trait;
use color_eyre::eyre::{anyhow, Error};
use k8s_openapi::api::core::v1::Secret;
use linode_rs::LinodeInstance;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const TOKEN_KEY: &str = "LINODE_TOKEN";
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct LinodeProvisioner {
    pub auth: String,
    pub region: String,
}

impl LinodeProvisioner {
    // gets token from Secret
    pub async fn get_token(&self, secret: Secret) -> color_eyre::Result<String> {
        let data = secret
            .data
            .ok_or_else(|| Error::msg("No data found in secret"))?;
        let token = data
            .get(TOKEN_KEY)
            .ok_or_else(|| Error::msg("No token found in secret"))?;

        let token = String::from_utf8(token.clone().0)?;
        Ok(token)
    }
}
