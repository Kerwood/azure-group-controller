set dotenv-load
cluster_name := "azure-group-controller"

[private]
default:
	@just --list

# Create a .env file to use for local development.
create-dot-env:
  cat .env-example >> .env

# Cargo run with needed parameters
run:
  cargo run serve -t $AZURE_TENANT_ID -i $AZURE_CLIENT_ID -s $AZURE_CLIENT_SECRET -b $RECONCILE_TIME -r $RETRY_TIME

# Bring up the Kind cluster
cluster-up:
	kind create cluster --name {{cluster_name}} --image kindest/node:v1.30.2
	sleep "10"
	kubectl wait --namespace kube-system --for=condition=ready pod --selector="tier=control-plane" --timeout=180s

# Bring down the Kind cluster
cluster-down:
	kind delete cluster --name {{cluster_name}}
	-rm ./kubeconfig

# Apply AzureGroup and AzureGroupManager CRDs
crds-apply:
  cargo run -p az-group-manager -- print-crd | kubectl apply -f -

# Delete AzureGroup and AzureGroupManager CRDs
crds-delete:
  kubectl delete crd azuregroupmanagers.kerwood.github.com azuregroups.kerwood.github.com

