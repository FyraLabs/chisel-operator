---
title: Linode
description: A guide of how to install Chisel Operator.
---

## Fields

| path   | type                    | description                                                                                             |
| ------ | ----------------------- | ------------------------------------------------------------------------------------------------------- |
| auth   | string                  | Name of the secret containing the Linode API token, under the `LINODE_TOKEN` secret key                                                      |
| region | string                  | Region ID of the Linode datacenter to provision the exit node in. See https://api.linode.com/v4/regions |
| size   | string? = "g6-nanode-1" | Size for the Linode. instance See https://api.linode.com/v4/linode/                                     |

## Examples

```yaml
apiVersion: chisel-operator.io/v1
kind: ExitNodeProvisioner
metadata:
  name: linode-provisioner
  namespace: default
spec:
  Linode:
    auth: linode-auth
    region: us-east
```
