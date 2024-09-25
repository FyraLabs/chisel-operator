---
title: Exposing a Service
description: A guide of how to get started with Chisel Operator.
---

Once you have a `ExitNode` or `ExitNodeProvisioner` set up in your cluster, you're ready to begin exposing services!

Here's an example service:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: whoami
  # annotations:
  # chisel-operator.io/exit-node-name: "my-exit-node"
spec:
  selector:
    app: whoami
  ports:
    - port: 80
      targetPort: 80
  type: LoadBalancer
```

As you can see, the type of this service is `LoadBalancer`, which is required for chisel-operator to pick up on the service.
Note that Chisel Operator acts on all LoadBalancer services in the cluster by default.

Additionally, there's also a commented out annotation, `chisel-operator.io/exit-node-name`.
By default, Chisel Operator will automatically select a random, unused `ExitNode` on the cluster if a cloud provisioner or exit node annotation is not set.
If you'd like to force the service to a particular exit node, you can uncomment out the annotation, setting it to the name of the `ExitNode` to target.

> As of Chisel Operator 0.4.0, you can now force multiple services to use the same exit node by setting the `chisel-operator.io/exit-node-name` annotation to the same value on each service, this can be useful by allowing you to group services together on the same exit node, saving resources by only running one exit node for multiple services.

<!-- TODO: cross namespace -->

Let's look at another example, this time using the automatic cloud provisioning functionality:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: whoami
  annotations:
    chisel-operator.io/exit-node-provisioner: "my-do-provisioner"
spec:
  selector:
    app: whoami
  ports:
    - port: 80
      targetPort: 80
  type: LoadBalancer
```

The only difference in the cloud case is the `chisel-operator.io/exit-node-provisioner` annotation, pointing to the name of the `ExitNodeProvisioner` resource you would like to use.

Chisel Operator will automatically use the specified provisioner to create a server in configured cloud, populating and managing a corresponding `ExitNode` resource in your cluster, which gets assigned to this service.

Please note that if the provisioner is in a different namespace than the service resource, you'll have to specify that in the annotation value.
For example, if the provisioner is in the `testing` namespace and has the name `my-do-provisioner`, the annotation value would be: `testing/my-do-provisioner`.

That's all for now!
