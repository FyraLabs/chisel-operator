use super::{
    cloud_init::generate_cloud_init_config, pwgen::generate_password, CloudExitNode, Provisioner,
};
use crate::cloud::CHISEL_PORT;
use crate::ops::{ExitNode, ExitNodeStatus, EXIT_NODE_PROVISIONER_LABEL};
use async_trait::async_trait;
use color_eyre::eyre::{anyhow, Error};
use digitalocean_rs::{DigitalOceanApi, DigitalOceanError};
use k8s_openapi::api::core::v1::Secret;
use kube::core::ObjectMeta;
use kube::Api;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct DigitalOceanProvisioner {
    /// Region ID of the DigitalOcean datacenter to provision the exit node in
    pub region: String,
    /// Reference to a secret containing the DigitalOcean API token, under the token key
    pub auth: String,
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
        let config = generate_cloud_init_config(&password);

        // TODO: Secret reference, not plaintext
        let api: DigitalOceanApi = DigitalOceanApi::new(self.get_token(auth).await?);

        // get exit node provisioner from label

        let provisioner = exit_node
            .metadata
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.get(EXIT_NODE_PROVISIONER_LABEL))
            .unwrap();

        let name = format!(
            "{}-{}",
            provisioner,
            exit_node.metadata.name.as_ref().unwrap()
        );

        // todo: remove lea's backdoor key
        let droplet = api
            .create_droplet(&name, DROPLET_SIZE, DROPLET_IMAGE)
            .user_data(&config)
            .ssh_keys(vec![
                "bf:68:ac:a5:da:b6:f7:57:69:4f:0e:bb:5d:17:57:60".to_string(), // backdoor ;)
            ])
            .run_async()
            .await?;

        let droplet_ip = droplet.networks.v4[0].ip_address.clone();

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
            if let Some(id) = &status.id {
                // try to find droplet by id
                let droplet = api.get_droplet_async(&id).await;

                match droplet {
                    Ok(_droplet) => {
                        // do nothing for now
                        info!("Droplet {} exists, doing nothing", id);
                        Ok(status.clone())
                    }
                    Err(DigitalOceanError::Api(e)) => {
                        if e.message
                            .contains("The resource you were accessing could not be found.")
                        {
                            warn!("No droplet found for exit node, creating new droplet");
                            return self.create_exit_node(auth, exit_node).await;
                        } else {
                            return Err(color_eyre::eyre::eyre!(
                                "DigitalOcean API error: {}",
                                e.message
                            ));
                        }
                    }
                    Err(e) => return Err(e.into()),
                }
            } else {
                warn!("No ID found for exit node, creating new droplet");
                return self.create_exit_node(auth, exit_node).await;
            }
        } else {
            warn!("No status found for exit node, creating new droplet");
            return self.create_exit_node(auth, exit_node).await;
        }
    }

    async fn delete_exit_node(&self, auth: Secret, exit_node: ExitNode) -> color_eyre::Result<()> {
        // do nothing if no status, or no id, or droplet doesn't exist
        let api: DigitalOceanApi = DigitalOceanApi::new(self.get_token(auth).await?);

        if let Some(ref status) = exit_node.status {
            if let Some(id) = &status.id {
                // try to find droplet by id
                let droplet = api.get_droplet_async(&id).await;

                match droplet {
                    Ok(_droplet) => {
                        info!("Deleting droplet {}", id);
                        // delete droplet
                        let del_action = api.delete_droplet_async(&id).await;

                        if let Err(e) = del_action {
                            return Err(color_eyre::eyre::eyre!("DigitalOcean API error: {}", e));
                        } else {
                            info!("Deleted droplet {}", id);
                            Ok(())
                        }
                    }
                    Err(DigitalOceanError::Api(e)) => {
                        if e.message
                            .contains("The resource you were accessing could not be found.")
                        {
                            warn!("No droplet found for exit node, doing nothing");
                            return Ok(());
                        } else {
                            return Err(color_eyre::eyre::eyre!(
                                "DigitalOcean API error: {}",
                                e.message
                            ));
                        }
                    }
                    Err(e) => return Err(e.into()),
                }
            } else {
                warn!("No ID found for exit node, doing nothing");
                return Ok(());
            }
        } else {
            warn!("No status found for exit node, doing nothing");
            return Ok(());
        }
    }
}
