use super::azure::GroupResponse;
use super::error::{Error, Result};
use az_group_crd::{AzureGroup, AzureGroupSpec, AzureGroupStatus};
use az_group_manager_crd::AzureGroupManager;
use chrono::prelude::*;
use kube::api::{Patch, PatchParams};
use kube::{api::ObjectMeta, Api, Resource};
// use tracing::{debug, error, info};

/// Creates an AzureGroup resource in Kubernetes using the provided `GroupResponse` and `AzureGroupManager`.
///
/// Arguments
///   * `g_response` - A reference to the `GroupResponse` containing the specifications for the Azure Group to be created.
///   * `manager` - A reference to the `AzureGroupManager` for getting owner reference and namepace.
///   * `k8s_client` - A reference to the `kube::Client` used to interact with the Kubernetes API server.
///
/// Returns a `Result<AzureGroup>` containing the Kubernetes resource.
pub async fn create_azure_group_resource(
    g_response: &GroupResponse,
    manager: &AzureGroupManager,
    k8s_client: &kube::Client,
) -> Result<AzureGroup> {
    let azure_spec: AzureGroupSpec = g_response.clone().try_into()?;
    let owner_ref = manager.controller_owner_ref(&()).unwrap();
    let namespace = manager
        .metadata
        .namespace
        .as_ref()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;

    let az_group_api = Api::<AzureGroup>::namespaced(k8s_client.clone(), namespace);

    let group_slug_name = g_response
        .slug_display_name()
        .ok_or_else(|| Error::MissingDisplayName(azure_spec.id.to_string()))?;

    let az_group = AzureGroup {
        metadata: ObjectMeta {
            name: Some(group_slug_name.clone()),
            owner_references: Some(vec![owner_ref.clone()]),
            ..ObjectMeta::default()
        },
        spec: azure_spec,
        status: Some(AzureGroupStatus {
            last_update: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }),
    };

    let result = patch_azure_group(&group_slug_name, &az_group_api, az_group).await?;

    return Ok(result);
}

async fn patch_azure_group(
    obj_name: &str,
    az_group_api: &Api<AzureGroup>,
    az_group: AzureGroup,
) -> Result<AzureGroup> {
    let result = az_group_api
        .patch(
            obj_name,
            &PatchParams::apply("azure-group-controller").force(),
            &Patch::Apply(&az_group),
        )
        .await
        .map_err(Error::AzureGroupCreationFailed)?;

    az_group_api
        .patch_status(obj_name, &PatchParams::default(), &Patch::Merge(&az_group))
        .await
        .map_err(Error::AzureGroupCreationFailed)?;

    return Ok(result);
}
