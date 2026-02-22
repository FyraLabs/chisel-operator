---
title: Introduction
description: An introduction to Chisel Operator and its features.
---

Chisel Operator is a lightweight L4 (Layer 4) tunneling operator for Kubernetes that lets you expose services publicly from restricted environments without VPNs, BGP, or complex networking.

It does this by leveraging [Chisel](https://github.com/jpillora/chisel), a lightweight L4 tunnel that initiates SSH-over-WebSocket connections from your cluster to a publicly-accessible exit node, securely forwarding traffic back to your services.

Chisel Operator manages the lifecycle of these tunnels and exit nodes, exposing them to other
Kubernetes resources as a standard Kubernetes [LoadBalancer](https://kubernetes.io/docs/concepts/services-networking/service/#loadbalancer) service. This allows you to use the same familiar Kubernetes APIs to expose your services, while Chisel Operator handles the complexity of managing the tunnels and exit nodes for you.
