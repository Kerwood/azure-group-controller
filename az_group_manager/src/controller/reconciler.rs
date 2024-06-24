use super::azure;
use super::error::{Error, Result};
use super::k8s;
use az_group_crd::AzureGroup;
use az_group_manager_crd::AzureGroupManager;
use futures::StreamExt;
use kube::{
    runtime::{
        controller::{Action, Controller},
        watcher,
    },
    Api, Client, ResourceExt,
};
use std::{sync::Arc, time::Duration};
use tracing::{debug, error};

#[derive(Debug, Clone)]
pub struct Args {
    pub az_tenant_id: String,
    pub az_client_id: String,
    pub az_client_secret: String,
    pub reconcile_time: u64,
    pub retry_time: u64,
}

#[derive(Clone)]
struct ReconcileContext {
    cli_args: Args,
    k8s_client: Client,
}

pub async fn run(cli_args: Args) -> Result<(), kube::Error> {
    let k8s_client = Client::try_default().await?;
    let azure_groups_manager_api = Api::<AzureGroupManager>::all(k8s_client.clone());
    let azure_groups_api = Api::<AzureGroup>::all(k8s_client.clone());
    let ctx = ReconcileContext { cli_args, k8s_client };

    Controller::new(azure_groups_manager_api.clone(), Default::default())
        .owns(azure_groups_api, watcher::Config::default())
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(ctx))
        .for_each(|_| futures::future::ready(()))
        .await;

    Ok(())
}

async fn reconcile(manager: Arc<AzureGroupManager>, ctx: Arc<ReconcileContext>) -> Result<Action> {
    debug!("running reconcile for manager object: {}", manager.name_any());

    match azure::get_members(&ctx.cli_args, &manager.spec.group_uid).await {
        Ok(group_response) => {
            k8s::create_azure_group_resource(&group_response, &manager, &ctx.k8s_client).await?
        }
        Err(err) => return Err(err),
    };

    Ok(Action::requeue(Duration::from_secs(ctx.cli_args.reconcile_time)))
}

fn error_policy(_object: Arc<AzureGroupManager>, err: &Error, ctx: Arc<ReconcileContext>) -> Action {
    error!(
        "{}, retrying in {} seconds.",
        err.to_string(),
        ctx.cli_args.retry_time
    );
    Action::requeue(Duration::from_secs(ctx.cli_args.retry_time))
}
