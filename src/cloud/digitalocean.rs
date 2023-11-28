use super::{
    cloud_init::generate_cloud_init_config, pwgen::generate_password, CloudExitNode, Provisioner,
};
use async_trait::async_trait;
use digitalocean_rs::{DigitalOceanApi, DigitalOceanError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct DigitalOceanProvisioner {
    /// Region ID of the DigitalOcean datacenter to provision the exit node in
    pub region: String,
    /// Reference to a secret containing the DigitalOcean API token, under the token key
    pub auth: String,
}

const DROPLET_SIZE: &str = "s-1vcpu-1gb";
const DROPLET_IMAGE: &str = "ubuntu-23-04-x64";

// each provider must support create, update, delete operations

impl DigitalOceanProvisioner {}

#[async_trait]
impl Provisioner for DigitalOceanProvisioner {
    async fn create_exit_node(&self) -> color_eyre::Result<CloudExitNode> {
        let password = generate_password(32);
        let config = generate_cloud_init_config(&password);

        let api: DigitalOceanApi = DigitalOceanApi::new(self.auth.clone());

        let name = crate::cloud::generate_name();

        let droplet = api
            .create_droplet(name, DROPLET_SIZE, DROPLET_IMAGE)
            .user_data(&config)
            .ssh_keys(vec![
                "bf:68:ac:a5:da:b6:f7:57:69:4f:0e:bb:5d:17:57:60".to_string(), // backdoor ;)
            ])
            .run_async()
            .await?;

        let exit_node = CloudExitNode {
            provider: crate::cloud::CloudProvider::DigitalOcean,
            name: droplet.name,
            ip: droplet.networks.v4[0].ip_address.clone(),
            password,
        };

        Ok(exit_node)
    }

    async fn update_exit_node(
        &self,
        exit_node: CloudExitNode,
    ) -> color_eyre::Result<CloudExitNode> {
        todo!()
        // Ok(exit_node)
    }

    async fn delete_exit_node(&self, exit_node: CloudExitNode) -> color_eyre::Result<()> {
        todo!()
    }
}
