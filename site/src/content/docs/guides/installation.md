---
title: Installation
description: A guide of how to install Chisel Operator.
---

Getting started with Chisel Operator is easy! We currently support the current (plus the last two versions) of Kubernetes. Kustomize is only supported at the moment, but a Helm chart is in progress.

## Kustomize
Install using the Kustomize config from the stable branch:

```bash
kubectl apply -k https://github.com/FyraLabs/chisel-operator?ref=stable
```

Or if you would like to go straight to the latest commit:

```bash
kubectl apply -k https://github.com/FyraLabs/chisel-operator
```

## Helm

To install using Helm, you can use the Chisel Operator Helm chart from the OCI registry:

```bash
helm install chisel-operator oci://ghcr.io/fyralabs/chisel-operator/chisel-operator
```

You can configure the helm chart values by creating a `values.yaml` file and passing it to the `helm install` command:

```bash
helm install chisel-operator oci://ghcr.io/fyralabs/chisel-operator/chisel-operator -f values.yaml
```

See [the Helm chart directory](https://github.com/FyraLabs/chisel-operator/tree/main/charts/chisel-operator) for more information on the Helm chart.
