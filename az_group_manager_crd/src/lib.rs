#[macro_use]
extern crate serde_derive;
use kube::{CustomResource, CustomResourceExt};
use schemars::JsonSchema;

#[derive(CustomResource, Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[kube(
    group = "kerwood.github.com",
    version = "v1",
    kind = "AzureGroupManager",
    namespaced,
    printcolumn = r#"{"name":"ID", "type":"string", "jsonPath":".spec.groupUid"}"#,
    printcolumn = r#"{"name":"Age", "type":"date", "jsonPath":".metadata.creationTimestamp"}"#
)]
#[kube(status = "AzureGroupManagerStatus")]
#[serde(rename_all = "camelCase")]
pub struct AzureGroupManagerSpec {
    pub group_uid: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AzureGroupManagerStatus {
    pub last_update: String,
}

pub fn print_crd() -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(&AzureGroupManager::crd())
}
