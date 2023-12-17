use super::{cloud_init::generate_cloud_init_config, pwgen::generate_password, Provisioner};
use crate::ops::{ExitNode, ExitNodeStatus, EXIT_NODE_PROVISIONER_LABEL};
use async_trait::async_trait;
use color_eyre::eyre::{anyhow, Error};
use digitalocean_rs::DigitalOceanApi;
use k8s_openapi::api::core::v1::Secret;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct DigitalOceanProvisioner {
    /// Region ID of the DigitalOcean datacenter to provision the exit node in
    /// If empty, DigitalOcean will randomly select a region for you, which might not be what you want
    #[serde(default)]
    pub region: String,
    /// Reference to a secret containing the DigitalOcean API token, under the token key
    pub auth: String,
    /// SSH key fingerprints to add to the exit node
    #[serde(default)]
    pub ssh_fingerprints: Vec<String>,
}

const DROPLET_SIZE: &str = "s-1vcpu-1gb";
const DROPLET_IMAGE: &str = "ubuntu-23-04-x64";

const TOKEN_KEY: &str = "DIGITALOCEAN_TOKEN";

// each provider must support create, update, delete operations

impl DigitalOceanProvisioner {
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

#[async_trait]
impl Provisioner for DigitalOceanProvisioner {
    async fn create_exit_node(
        &self,
        auth: Secret,
        exit_node: ExitNode,
    ) -> color_eyre::Result<ExitNodeStatus> {
        let password = generate_password(32);

        // create secret for password too

        let _secret = exit_node.generate_secret(password.clone()).await?;

        let config = generate_cloud_init_config(&password, exit_node.spec.port);

        // TODO: Secret reference, not plaintext
        let api: DigitalOceanApi = DigitalOceanApi::new(self.get_token(auth).await?);

        // get exit node provisioner from label

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

        let name = format!(
            "{}-{}",
            provisioner,
            exit_node.metadata.name.as_ref().unwrap()
        );

        let droplet = {
            let mut droplet = api
                .create_droplet(&name, DROPLET_SIZE, DROPLET_IMAGE)
                .user_data(&config)
                .ssh_keys(self.ssh_fingerprints.clone())
                .tags(vec![format!("chisel-operator-provisioner:{}", provisioner)]);

            if !self.region.is_empty() {
                droplet = droplet.region(&self.region);
            }

            droplet.run_async().await?
        };

        // now that we finally got the thing, now keep polling until it has an IP address

        let droplet_id = droplet.id.to_string();

        let droplet_ip = loop {
            let droplet = api.get_droplet_async(&droplet_id).await?;

            debug!(?droplet, "Getting droplet data");

            if let Some(droplet_public_net) =
                droplet.networks.v4.iter().find(|net| net.ntype == "public")
            {
                let droplet_ip = droplet_public_net.ip_address.clone();
                break droplet_ip;
            } else {
                warn!("Waiting for droplet to get IP address");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        };

        let exit_node = ExitNodeStatus {
            name: name.clone(),
            ip: droplet_ip.clone(),
            id: Some(droplet.id.to_string()),
            provider: provisioner.clone(),
        };

        Ok(exit_node)
    }

    async fn update_exit_node(
        &self,
        auth: Secret,
        exit_node: ExitNode,
    ) -> color_eyre::Result<ExitNodeStatus> {
        // check if droplet exists, then update it
        let api: DigitalOceanApi = DigitalOceanApi::new(self.get_token(auth.clone()).await?);
        let node = exit_node.clone();

        if let Some(ref status) = &exit_node.status {
            let droplet_id = status.id.as_ref().ok_or_else(|| {
                anyhow!(
                    "No droplet ID found in status for exit node {}",
                    node.metadata.name.as_ref().unwrap()
                )
            })?;

            let droplet = api.get_droplet_async(&droplet_id).await?;

            let mut status = status.clone();

            if let Some(ip) = droplet.networks.v4.iter().find(|net| net.ntype == "public") {
                status.ip = ip.ip_address.clone();
            }

            Ok(status)
        } else {
            warn!("No status found for exit node, creating new droplet");
            // TODO: this should be handled by the controller logic
            return self.create_exit_node(auth, exit_node).await;
        }
    }

    async fn delete_exit_node(&self, auth: Secret, exit_node: ExitNode) -> color_eyre::Result<()> {
        // do nothing if no status, or no id, or droplet doesn't exist
        let api: DigitalOceanApi = DigitalOceanApi::new(self.get_token(auth).await?);
        let droplet_id = exit_node
            .status
            .as_ref()
            .and_then(|status| status.id.as_ref());

        if let Some(droplet_id) = droplet_id {
            info!("Deleting droplet with ID {}", droplet_id);
            api.delete_droplet_async(droplet_id).await?;
        }
        Ok(())
    }
}
