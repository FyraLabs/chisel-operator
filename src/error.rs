use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReconcileError {
    #[error("Kube Error: {0}")]
    KubeError(#[from] kube::Error),

    #[error("There are no exit nodes available to assign")]
    NoAvailableExitNodes,

    #[error("There are no ports set on this LoadBalancer")]
    NoPortsSet,

    #[error("The provided cloud provisioner was not found in the cluster")]
    CloudProvisionerNotFound,
}
