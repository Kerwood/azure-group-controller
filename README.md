# Azure Group Controller

[![forthebadge made-with-rust](http://ForTheBadge.com/images/badges/made-with-rust.svg)](https://www.rust-lang.org/)

A Kubernetes controller that creates a `AzureGroup` resource with a list of members and some basic information on the group.
The controller will continuously reonconsile the `AzureGroup` resource.

## Prerequisites

For deploying:

- Helm, <https://helm.sh/docs/intro/install/>
- Kubectl, <https://kubernetes.io/docs/tasks/tools/>

For developing:

- Kind, <https://kind.sigs.k8s.io/docs/user/quick-start/>
- Rust, <https://www.rust-lang.org/tools/install>
- Just, <https://github.com/casey/just>

## Install

Start be creating a default App Registration in your Azure tenant. Don't chose any Platform, just give it a name.  
Add the `GroupMember.Read.All` and `User.ReadBasic.All` Application Permissions to the App Registration and create a new client secret.
If the maximun expiration date of years is not enough for you, use the `az` cli to set as many years as you want.

Create a Kubernetes secret in your cluster containing the Azure tenant, client ID and secret.

```
kubectl create secret generic az-group-manager \
  --from-literal=tenant-id=<uuid> \
  --from-literal=client-id=<uuid> \
  --from-literal=client-secret=<secret>
```

Add the Helm repository and update.

```
helm repo add kerwood https://kerwood.github.io/helm-charts
helm repo update
```

Install the controller.

```
helm install az-group-manager kerwood/az-group-manager --namespace <namespace>
```

## How to use it

Create a `AzureGroupManager` resources with the UUID of a Azure Group in the spec.

```yaml
apiVersion: kerwood.github.com/v1
kind: AzureGroupManager
metadata:
  name: my-azure-group-name-can-be-anything
spec:
  groupUid: 00b9c3c9-09d1-4e58-bd89-ec3ebcfb47e6
```

The controller will create a child resource with the group information.

```yaml
apiVersion: kerwood.github.com/v1
kind: AzureGroup
metadata:
  name: planet-express
  ...
spec:
  count: 2
  description: Best delivery boys in the business
  displayName: Planet Express
  id: 00b9c3c9-09d1-4e58-bd89-ec3ebcfb47e6
  mail: planet.express@futurama.com
  members:
  - displayName: Bender Rodriguez
    id: 814a8ea1-c2d5-47ca-8f80-c838646536c3
    mail: bender.rodriguez@futurama.com
  - displayName: Philip J. Fry
    id: 631fd65d-7d72-4969-a6e5-56ad6062485f
    mail: philip.j.fry@futurama.com
```

# CLI

```
Usage: az_group_manager serve [OPTIONS] --az-tenant-id <AZ_TENANT_ID> --az-client-id <AZ_CLIENT_ID> --az-client-secret <AZ_CLIENT_SECRET>

Options:
  -t, --az-tenant-id <AZ_TENANT_ID>
          Azure Tenant ID. [env: AZ_TENANT_ID=]
  -i, --az-client-id <AZ_CLIENT_ID>
          Azure App Registration Client ID. [env: AZ_CLIENT_ID=]
  -s, --az-client-secret <AZ_CLIENT_SECRET>
          Azure App Registration Client Secret. [env: AZ_CLIENT_SECRET=]
      --structured-logs
          Logs will be output as JSON. [env: STRUCTURED_LOGS=]
  -b, --reconcile-time <RECONCILE_TIME>
          Seconds between each reconciliation. [env: RECONCILE_TIME=] [default: 300]
  -l, --log-level <LOG_LEVEL>
          Log Level. [env: LOG_LEVEL=] [default: info] [possible values: trace, debug, info, warn, error]
  -r, --retry-time <RETRY_TIME>
          Seconds between each retry if reconciliation fails. [env: RETRY_TIME=] [default: 10]
  -h, --help
          Print help
```
