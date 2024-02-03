---
title: DigitalOcean
description: A guide of how to install Chisel Operator.
---

## Fields

| path             | type                    | description                                                                                                                                                                                           |
| ---------------- | ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| auth             | string                  | Reference to a secret containing the DigitalOcean API token, under the `DIGITALOCEAN_TOKEN` secret key                                                                                                |
| region           | string?                 | Region ID of the DigitalOcean datacenter to provision the exit node in. If empty, DigitalOcean will randomly select a region for you, which might not be what you want. See https://slugs.do-api.dev/ |
| size             | string? = "s-1vcpu-1gb" | Size for the DigitalOcean droplet. See https://slugs.do-api.dev/                                                                                                                                      |
| ssh_fingerprints | string[]? = []          | SSH key fingerprints to add to the exit node                                                                                                                                                          |

## Example

```yaml
apiVersion: chisel-operator.io/v1
kind: ExitNodeProvisioner
metadata:
  name: digitalocean-provisioner
  namespace: default
spec:
  DigitalOcean:
    auth: digitalocean-auth
    region: nyc2
```
