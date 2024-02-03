---
title: ExitNode
description: A reference page about the ExitNode resource.
---

An `ExitNode` is a resource representing a Chisel exit node that the operator can use for tunneling.
It contains the configuration required to connect to the remote Chisel server.

## Fields

| path          | type             | description                                                                                                                                                                        |
| ------------- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| auth          | string?          | Optional authentication secret name to connect to the control plane                                                                                                                |
| chisel_image  | string?          | Optional value for the chisel client image used to connect to the chisel server If not provided, jpillora/chisel:latest is used                                                    |
| default_route | boolean? = false | Optional boolean value for whether to make the exit node the default route for the cluster If true, the exit node will be the default route for the cluster default value is false |
| external_host | string?          | Optional real external hostname/IP of exit node If not provided, the host field will be used                                                                                       |
| fingerprint   | string?          | Optional but highly recommended fingerprint to perform host-key validation against the server's public key                                                                         |
| host          | string           | Hostname or IP address of the chisel server                                                                                                                                        |
| port          | uint16           | Control plane port of the chisel server                                                                                                                                            |

## Annotations

| name                                     | description                                                                                                                                                                      |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| chisel-operator.io/exit-node-provisioner | The exit node provisioner to use to provision this node. Example: "default/my-exit-node-provisioner". Most users won't need to use this unless they want to pre-provision nodes. |

## Examples

```yaml
apiVersion: chisel-operator.io/v1
kind: ExitNode
metadata:
  name: my-exit-node
  namespace: default
spec:
  # IP address of exit node
  host: "192.168.1.1"
  # Control plane socket port
  port: 9090
  # Name of the secret containing the auth key
  # Create a secret with a key named "auth" and put the value there
  # auth: SECRET-NAME
```

```yaml
apiVersion: chisel-operator.io/v1
kind: ExitNode
metadata:
  name: my-exit-node
  namespace: default
  annotations:
    chisel-operator.io/exit-node-provisioner: "digitalocean"
spec:
  # IP address of exit node
  host: ""
  # Control plane socket port
  port: 9090
  # Name of the secret containing the auth key
  # Create a secret with a key named "auth" and put the value there
  # auth: SECRET-NAME
```
