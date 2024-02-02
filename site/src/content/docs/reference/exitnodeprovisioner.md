---
title: ExitNodeProvisioner
description: A reference page about the ExitNodeProvsioner resource.
---

An ExitNodeProvisioner is a resource representing an external provider that Chisel Operator can use to automatically provision exit nodes.
It contains the configuration required to provision nodes on the external provider.

## Fields

The fields of the ExitNodeProvisioner are dependent on the provider you would like to use.
To see the fields for a provisioner, please see the provider-specific documentation.

## Examples

```yaml
apiVersion: chisel-operator.io/v1
kind: ExitNodeProvisioner
metadata:
  name: digitalocean-provisioner
  namespace: default
spec:
  DigitalOcean:
    auth: digitalocean-auth
    region: sgp1
```

```yaml
apiVersion: chisel-operator.io/v1
kind: ExitNodeProvisioner
metadata:
  name: linode-provider
  namespace: default
spec:
  Linode:
    auth: linode-auth
    region: us-east
```
