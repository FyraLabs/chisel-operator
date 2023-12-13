# Chisel Kubernetes Operator ⚒️

Use a VPS (or any other machine) as a reverse proxy for your Kubernetes cluster, without paying the extra 25$ a month!

This is a Kubernetes operator for Chisel. It allows you to use Chisel as a LoadBalancer provider for your Kubernetes cluster, similar to [inlets-operator](https://github.com/inlets/inlets-operator)

## Features

- Automatic provisioning of exit nodes on cloud providers
- Free and open-source
- TCP and UDP support
- **INFINITE** tunnels! (You can create as many tunnels as you want, as long as you have enough bandwidth)
- Use any machine as an exit node
- Hybrid cloud support (You can use multiple cloud providers at once)

## TODO

- [x] Authentication
- [x] Multiple tunnel services per exit node (so you don't have to pay for multiple VMs)
- [x] Extra configuration options
- [x] TCP/UDP support
- [ ] Multiple IPs per exit node
- [x] Multiple exit nodes support
- [x] Cloud provisioner (like inletsctl/inlets-operator)

## Why?

### The issue

If you want to expose a service to the internet, you need a public IP address.

However, if you're running a cluster inside a NATed network (like a home network), you can't just expose your service to the internet. You need to port forward your service to the internet. This might be fine, but then there's another problem:

The world's running out of IPv4 addresses. This means that ISPs are starting to charge extra for public IP addresses, and most home networks are locked behind a CGNAT, and requires you to pay extra for a public IP address.

You could just use an IPv6 address, but most ISPs don't support IPv6 yet, and K8s with IPv6 is kind of a pain to set up.

### The other issue

You might say, "What about Inlets?" Inlets is a great solution, but it comes with a couple caveats:

- You need to pay for an inlets PRO license to even use it (It's a proprietary solution)
- You still need to pay for the exit node on top of the inlets PRO license

### The solution

Introducing the Fyra Labs Chisel Operator! This operator provides a replacement for inlets, but free and open-source!

This operator makes use of the [Chisel] tunnel to expose your services to the internet through SSH. And you can use any machine as an exit node!

[Chisel]: https://github.com/jpillora/chisel

Since Chisel routes traffic through SSH, all traffic is encrypted and secure. The Chisel Operator also supports automatic provisioning of exit nodes on cloud providers, so you get basically the same functionality, but free!

---

While this code is free and open-source, we still accept donations! If you really like this project, please consider donating to us on [GitHub Sponsors](https://github.com/sponsors/FyraLabs) :3

## How does it work?

This operator works similarly to inlets-operator. It watches for `LoadBalancer` services, then allocates an exit node's IP address for that service and creates a Chisel client deployment on that node. The Chisel client will then connect to the Chisel server running on the exit node, and the service will be exposed on the exit node's IP address.

## Alternatives

### SaaS solutions

- [Cloudflare (Argo) Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-apps) - Cloudflare's solution to this problem. It is free and also open-source, but it only works on Cloudflare-managed domains and requires you to use Cloudflare's DNS service. But it comes with a couple caveats:
  - Only HTTP and HTTPS is supported for exposing services. If you want to expose a TCP service, you must connect to it through Cloudflare Tunnel on the client.
  - According to Cloudflare's [Terms of Service](https://www.cloudflare.com/terms/), you are not allowed to use Cloudflare's proxies to stream video or audio content. This means that you cannot use Cloudflare Tunnel to expose your Plex or Jellyfin server, or any other media streaming service. This is also the reason I started this project.
- [ngrok](https://ngrok.com/) - ngrok is a proprietary solution that allows you to expose your local service to the internet. It is free to use, but it comes with a couple caveats:
  - Only HTTP and HTTPS is supported for exposing services. TCP traffic is supported through a paid plan.
  - Limited bandwidth
  - Custom domains are only available on a paid plan

### Self-hosted solutions

- Run Chisel manually on your exit node - This is the most straightforward solution. You can simply run Chisel manually on your exit node without using this operator. However, this solution is hard to automate, which is the point of this project.
- [frp](https://github.com/fatedier/frp) - Fast reverse proxy, requires manual configuration of server and client.
- [inlets](https://inlets.dev/) - Bite the bullet and pay for an inlets PRO license. inlets-pro allows you to automatically provision exit nodes on cloud providers, but it is a proprietary solution and requires you to pay a monthly fee.
- [rathole](https://github.com/rapiz1/rathole) - Similar to frp, written in Rust.

### VPNs and overlay networks

- [Tailscale](https://tailscale.com/) - VPN solution that allows you to connect your devices in one big overlay network. Also has Funnel, a reverse proxy solution that allows you to expose your local service to the internet. Self-hostable control plane is available, but default is to use Tailscale's hosted control plane.
- ZeroTier - Similar to Tailscale, Under BSD license, Can connect to multiple networks at once.

---

Find more alternatives [here](https://github.com/anderspitman/awesome-tunneling)

## How do I use it?

There are two ways to use this operator:

- Manually provision the exit (reverse proxy) node, and let the operator manage the Chisel client deployment
- Let the operator provision exit nodes on a cloud provider of your choice. The operator currently supports the following cloud providers:
  - DigitalOcean
  - Linode (Currently only on regions with Metadata services)
  - AWS

## Cluster Installation

Install using the Kustomize config:

```bash
kubectl apply -k https://github.com/FyraLabs/chisel-operator
```

A Helm chart will be available soon.

## Usage

### Operator-managed exit nodes

This operator can automatically provision exit nodes on cloud providers.

To use this feature, you must first create a `ExitNodeProvisioner` resource. This resource contains the configuration for the cloud provider, and the operator will use this resource to provision exit nodes.

```yaml
apiVersion: chisel-operator.io/v1
kind: ExitNodeProvisioner
metadata:
  name: digitalocean
  namespace: default
spec:
  # Cloud provider configuration, must be one per resource
  # Valid values are DigitalOcean, Linode, AWS
  DigitalOcean:
    # Reference to a secret containing the DigitalOcean API token
    # with key `DIGITALOCEAN_TOKEN`
    # Must be in the same namespace as the ExitNodeProvisioner
    auth: digitalocean
    region: sgp1
```

Now, you can go with one of the two routes:

#### Automatic provisioning per service

Chisel Operator can automatically allocate cloud exit nodes for services,
if you set an annotation on a `LoadBalancer` service.

```yaml
apiVersion: v1
kind: Service
metadata:
  name: whoami
  annotations:
    chisel-operator.io/exit-node-provider: "digitalocean"
spec:
  selector:
    app: whoami
  ports:
    - port: 80
      targetPort: 80
  type: LoadBalancer
```

This will create a new `ExitNode` resource named after the service, and the operator will automatically allocate an exit node for that service.

This is useful if you want to just allocate an entire exit node for a single service.

#### Manually-allocated, but operator-managed exit nodes

You can also manually allocate exit nodes, but still let the operator manage the Chisel client deployment. This is useful if you want to allocate a single exit node for multiple services, in case you're on a budget and don't want to pay for multiple exit nodes for each service.

To do this, create an `ExitNode` resource with the annotation `chisel-operator.io/exit-node-provider` set to the name of the `ExitNodeProvisioner` resource.

```yaml
apiVersion: chisel-operator.io/v1
kind: ExitNode
metadata:
  name: my-exit-node
  namespace: default
  annotations:
    chisel-operator.io/exit-node-provider: "digitalocean"
spec:
  # IP address of exit node
  # In this case, we will leave this as a blank string, and let the operator allocate an IP address for us
  host: ""
  # Control plane socket port
  port: 9090
  # Name of the secret containing the auth key
  # This is not required, but recommended
  # If not set, the operator will automatically generate a secret for you
  # auth: cloud-test-auth
```

Now, to use this exit node, you can create a `LoadBalancer` service with the annotation `chisel-operator.io/exit-node-name` set to the name of the `ExitNode` resource.

```yaml
apiVersion: v1
kind: Service
metadata:
  name: whoami
  annotations:
    chisel-operator.io/exit-node-name: "cloud-test"
spec:
  selector:
    app: whoami
  ports:
    - port: 80
      targetPort: 80
  type: LoadBalancer
```

> NOTE: You can also use this for manually-provisioned exit nodes

> NOTE: If you do not specify the annotation, the operator will allocate the first available exit node for you.

### Provisioning Chisel manually

> NOTE: You can skip this step if you're using the cloud provisioner.

To install Chisel, install the Chisel binary on the machine using this script:

```bash
curl https://i.jpillora.com/chisel! | bash
```

**OPTIONAL:** You should create a systemd service for Chisel so it can run in the background. Create a file called `/etc/systemd/system/chisel.service` with the following contents:

```ini
[Unit]
Description=Chisel Tunnel
Wants=network-online.target
After=network-online.target
StartLimitIntervalSec=0

[Install]
WantedBy=multi-user.target


[Service]
Restart=always
RestartSec=1
User=root
# You can add any additional flags here
# This example uses port 9090 for the tunnel socket. `--reverse` is required for our use case.
ExecStart=/usr/local/bin/chisel server --port=9090 --reverse
# Additional .env file for auth and secrets
EnvironmentFile=-/etc/sysconfig/chisel
```

For security purposes, you should create a `.env` file at `/etc/sysconfig/chisel` (literally) with the following contents:

```bash
# This is the root credentials for the Chisel server. You can change this to whatever you want. Just make sure to keep it a secret.
# You can also use the `--authfile` argument in the ExecStart command instead of this, for a custom ACL file (in JSON).
AUTH=user:password
```

Then run `systemctl daemon-reload` and `systemctl enable --now chisel.service` to enable and start the service.

## Best Practices

You should always secure your Chisel server with a username and password. You can authenticate to the server by creating a secret in the same namespace as the `ExitNode` with a key called `auth`, and setting the `auth` field in the `ExitNode` to the name of the secret. The secret should be a string of `username:password` in plain text.

Currently, you should use the public IP address of your exit node as the `host` field in the `ExitNode` resource. This is because the operator currently does not support using a domain name as the `host` field. This will be fixed in the future.

### Exposing services

It is recommended you use an Ingress controller to expose your services. This greatly simplifies the process for exposing other services, as you only need to expose the Ingress controller's HTTP(S) ports.

## How do I contribute?

Feel free to open a pull request or an issue if you have any suggestions or improvements. We're open to any ideas!
