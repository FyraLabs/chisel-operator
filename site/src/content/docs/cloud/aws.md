---
title: AWS
description: A guide of how to install Chisel Operator.
---

## Fields

| path           | type                 | description                                                                                                                                               |
| -------------- | -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| auth           | string               | Reference to a secret containing the AWS access key ID and secret access key, under the `access_key_id` and `secret_access_key` secret keys               |
| region         | string               | Region ID for the AWS region to provision the exit node in. See https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/using-regions-availability-zones.html |
| security_group | string?              | Security group name to use for the exit node, uses the default security group if not specified                                                            |
| size           | string? = "t2.micro" | Size for the EC2 instance. See https://aws.amazon.com/ec2/instance-types/                                                                                 |

## Examples

```yaml
apiVersion: chisel-operator.io/v1
kind: ExitNodeProvisioner
metadata:
  name: aws-provisioner
  namespace: default
spec:
  AWS:
    auth: aws-auth
    region: us-east-1
---
apiVersion: v1
kind: Secret
metadata:
  name: aws-auth
  namespace: default
type: Opaque
stringData:
  access_key_id: xxxxx
  secret_access_key: xxxxx
```
