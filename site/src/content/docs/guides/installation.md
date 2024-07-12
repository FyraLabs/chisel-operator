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

Yup, that's it.
