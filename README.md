# Chisel Kubernetes Operator

This is a Kubernetes operator for Chisel. It allows you to use Chisel as a LoadBalancer provider for your Kubernetes cluster, similar to [inlets-operator](https://github.com/inlets/inlets-operator)

## Why?

This project was started due to my frustration with inlets' business model.

inlets used to be an open-source reverse-proxy-over-WebSockets solution that lets you expose your local service to an exit node in the cloud. It was a great solution for people who do not have a public IP address, or simply want to expose their service to the internet without having to deal with port-forwarding and NAT.

However, inlets has recently switched to a closed-source model, where the only way to use it is to pay for an inlets PRO license. This is a huge turn-off for me, and many others, as it is no longer a viable solution for hobbyists and small businesses on a budget.

Even worse, OpenFaaS has taken over the inlets project and has deleted the source code for the open-source version of inlets. This is a huge red flag for me, as it means that this project is no longer open-source, and is now a proprietary solution.

I do not want to pay a 25$ monthly fee on top of the reverse proxy VPS that I am already paying for, so I decided to make my own solution.

This project will never have a profit incentive, and will always be open-source. I simply want to make a solution that is FOSS, and share it with the world because I believe that it will be useful to many people.

I myself work at a financially struggling startup, and also live in a country where the average salary is 500$ a month. I understand the struggle of having to pay for expensive software, and I want to make a solution that is free for everyone to use. Having to rent out a VPS for 5-10$ a month is already expensive enough, and I don't want to have to pay an additional 25$ a month just to use a reverse proxy so I can expose my pirated content to the Internet.

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
- 
---
Find more alternatives [here](https://github.com/anderspitman/awesome-tunneling)


## How do I use it?

Currently, you will need to manually provision a Chisel server on your own exit node.

MORE INSTRUCTIONS COMING SOON

## How do I contribute?

Feel free to open a pull request or an issue if you have any suggestions or improvements. I'm open to any ideas!
