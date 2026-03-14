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

## Limiting reconciliation to a LoadBalancer class

If you only want the operator to manage services that opt in to a specific load balancer class, you can set the `LOAD_BALANCER_CLASS` environment variable (or the `loadBalancerClass` Helm value) when deploying it.
Once the operator is running with that filter, only services whose `spec.loadBalancerClass` matches the configured class name will be reconciled; other services are ignored.

To opt a service in, add the same `loadBalancerClass` to the Service spec:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: whoami
spec:
  type: LoadBalancer
  loadBalancerClass: my.chisel.class
  selector:
    app: whoami
  ports:
    - port: 80
      targetPort: 80
```

Leaving the value empty (or omitting the field) continues to expose the service through any unfiltered operator instance.

## Selecting a specific exit node

By default, Chisel Operator will automatically select an available `ExitNode` on the cluster if no exit node annotation is set.
If you'd like to force the service to use a particular exit node, you can set the `chisel-operator.io/exit-node-name` annotation to the name of the `ExitNode` to target.

```yaml
apiVersion: v1
kind: Service
metadata:
  name: whoami
  annotations:
    chisel-operator.io/exit-node-name: "my-exit-node"
spec:
  selector:
    app: whoami
  ports:
    - port: 80
      targetPort: 80
  type: LoadBalancer
```

> **Note:** As of Chisel Operator 0.4.0, you can force multiple services to use the same exit node by setting the `chisel-operator.io/exit-node-name` annotation to the same value on each service. This allows you to group services together on the same exit node, saving resources by only running one exit node for multiple services.

### Cross-namespace exit node selection

If your `ExitNode` is in a different namespace than your service, you must specify the namespace in the annotation value using the format `namespace/name`.

For example, if you have an `ExitNode` named `shared-exit` in the `infrastructure` namespace, and your service is in the `default` namespace:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: whoami
  namespace: default
  annotations:
    chisel-operator.io/exit-node-name: "infrastructure/shared-exit"
spec:
  selector:
    app: whoami
  ports:
    - port: 80
      targetPort: 80
  type: LoadBalancer
```

If you omit the namespace prefix, the operator will look for the exit node in the same namespace as the service.

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

## Operator logging

The operator defaults to `logfmt`. If you're running on Kubernetes and want JSON logs for easy ingestion into Loki, VictoriaLogs, or similar systems, set `LOGGER` to `json`.

You can override this behavior with the `LOGGER` environment variable (valid values: `logfmt`, `pretty`, `json`, `compact`).
