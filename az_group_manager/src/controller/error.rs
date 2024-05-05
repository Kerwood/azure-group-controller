#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("reqwest:Error [ {0} ]")]
    Reqwest(#[from] reqwest::Error),

    #[error("azure_core::Error [ {0} ]")]
    AzureCore(#[from] azure_core::Error),

    #[error("url::ParseError [ {0} ]")]
    URLParse(#[from] url::ParseError),

    #[error("kube::Error [ {0} ]")]
    KubeRS(#[from] kube::Error),

    #[error("Namespace is missing from AzureGroup resource {0}.")]
    NamespaceMissing(String),

    #[error("could not convert GroupResponseMember into Member. {0}")]
    IntoMemberFailed(String),

    #[error("could not convert GroupResponse into AzureGroupSpec. {0}")]
    IntoAzureGroupSpecFailed(String),

    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),

    #[error("GroupResponse is missing display_name propety: {0}")]
    MissingDisplayName(String),

    #[error("AzureGroupCreationFailed: {0}")]
    AzureGroupCreationFailed(#[source] kube::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
