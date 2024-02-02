---
title: Using Cloud Provisioning
description: A guide of how you can use cloud provisioning to automatically setup exit nodes.
---

Chisel Operator makes it easy to integrate into your preferred cloud provider using our exit node provisioning functionality.
Let's look at an example of how this works.

First, we'll want to create an `ExitNodeProvisioner` resource.
For this guide, I'll be using DigitalOcean, but if you'd like to use a different provider, please look at the reference for the provisioner's config:

```yaml
apiVersion: chisel-operator.io/v1
kind: ExitNodeProvisioner
metadata:
  name: digitalocean
  namespace: default
spec:
  DigitalOcean:
    auth: digitalocean-auth
    region: sgp1
```

Most provisioners will also require a form of authentication.
In the case of DigitalOcean, you need a personal access token with read/write permissions, which can be created in the API tab of the dashboard.

Next, we'll create a secret with our token, using the secret key expected for the provisioner, in this case `DIGITALOCEAN_TOKEN`:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: digitalocean-auth
  namespace: default
type: Opaque
stringData:
  DIGITALOCEAN_TOKEN: xxxxx
```

And, that's it, we're ready for provisioning!
