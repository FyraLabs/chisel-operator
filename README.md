# Chisel Kubernetes Operator

This is a Kubernetes operator for Chisel. It allows you to use Chisel as a LoadBalancer provider for your Kubernetes cluster, similar to [inlets-operator](https://github.com/inlets/inlets-operator)

## TODO

- [x] Authentication
- [x] Multiple tunnel services per exit node (so you don't have to pay for multiple VMs)
- [ ] Extra configuration options
- [x] TCP/UDP support
- [ ] Multiple IPs per exit node
- [ ] Multiple exit nodes support
- [ ] Cloud provisioner (like inletsctl/inlets-operator)

## Why?

This project was started due to my frustration with inlets' business model.

inlets used to be an open-source reverse-proxy-over-WebSockets solution that lets you expose your local service to an exit node in the cloud. It was a great solution for people who do not have a public IP address, or simply want to expose their service to the internet without having to deal with port-forwarding and NAT.

However, inlets has recently switched to a closed-source model, where the only way to use it is to pay for an inlets PRO license. This is a huge turn-off for me, and many others, as it is no longer a viable solution for hobbyists and small businesses on a budget.

I do not want to pay a 25$ monthly fee on top of the reverse proxy VPS that I am already paying for, so I decided to make my own solution.

This project will never have a profit incentive, and will always be open-source. I simply want to make a solution that is FOSS, and share it with the world because I believe that it will be useful to many people.

I myself work at a financially struggling startup, and also live in a country where the average salary is 500$ a month. I understand the struggle of having to pay for expensive software, and I want to make a solution that is free for everyone to use. Having to rent out a VPS for 5-10$ a month is already expensive enough, and I don't want to have to pay an additional 25$ a month just to use a reverse proxy so I can expose my content to the Internet.

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

Currently, you will need to manually provision a Chisel server on your own exit node.

## Cluster Installation

Install using the Kustomize config:
```bash
kubectl apply -k https://github.com/FyraLabs/chisel-operator
```

### Deploying the operator

First, you will need a VPS with a public IP address that will act as your exit node. You can use any cloud provider you want. Here are some suggestions:

- [DigitalOcean](https://www.digitalocean.com/)
- [Vultr](https://www.vultr.com/)
- [Linode](https://www.linode.com/)
- [Google Cloud](https://cloud.google.com/)
- [Hetzner](https://www.hetzner.com/)
- [Contabo](https://contabo.com/)

After purchasing a VPS, you will need to provision Chisel on it.

### Provisioning Chisel

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

```env
# This is the root credentials for the Chisel server. You can change this to whatever you want. Just make sure to keep it a secret.
# You can also use the `--authfile` argument in the ExecStart command instead of this, for a custom ACL file (in JSON).
AUTH=user:password
```

Then run `systemctl daemon-reload` and `systemctl enable --now chisel.service` to enable and start the service.

### Deploying the operator

**NOTE:** This operator is currently in development, breaking changes may occur at any time. It's not ready for production use yet.

To install the operator, deploy the kustomization config from this repository:

```bash
kubectl apply -k https://github.com/FyraLabs/chisel-operator
```

### Setting up and usage

Create an `ExitNode` resource with the information gained from the previous steps:

```yaml
apiVersion: chisel-operator.io/v1
kind: ExitNode
metadata:
  name: my-reverse-proxy
spec:
  host: 192.168.0.1 # Bring your own IP
  port: 9090 # The port you set in the service
  # securing the connection is optional, but recommended
  # auth: # The name of a secret containing username and password keys
```

To use this operator, create a `LoadBalancer` service with no `spec.loadBalancerClass` field or with `spec.loadBalancerClass: "chisel-operator.io/chisel-operator-class"`. The operator will then deploy a Chisel client for that service and expose it on the exit node's IP address. The operator will then manage the service's external IPs and status, and you should be able to use the service as if it was any other LoadBalancer service.

```yaml
apiVersion: v1
kind: Service
metadata:
  name: my-service
spec:
  type: LoadBalancer
  ports:
    - port: 80
      targetPort: 8080
  selector:
    app: my-app
  # loadBalancerClass: chisel-operator.io/chisel-operator-class # Optional, if you're using multiple LoadBalancer operators
```

MORE INSTRUCTIONS COMING SOON

## Best Practices

You should always secure your Chisel server with a username and password. You can authenticate to the server by creating a secret in the same namespace as the `ExitNode` with a key called `auth`, and setting the `auth` field in the `ExitNode` to the name of the secret. The secret should be a string of `username:password` in plain text.

Currently, you should use the public IP address of your exit node as the `host` field in the `ExitNode` resource. This is because the operator currently does not support using a domain name as the `host` field. This will be fixed in the future.

### Exposing services

It is recommended you use an Ingress controller to expose your services. This greatly simplifies the process for exposing other services, as you only need to expose the Ingress controller's HTTP(S) ports. 


## How do I contribute?

Feel free to open a pull request or an issue if you have any suggestions or improvements. I'm open to any ideas!
