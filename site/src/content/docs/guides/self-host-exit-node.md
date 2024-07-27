---
title: Self-hosting an Exit Node
description: A guide of how you can self-host your own Chisel exit node.
---

First, you'll need a machine where you can run [Chisel](https://github.com/jpillora/chisel), the software that Chisel Operator uses to tunnel to your server.
We assume that you're running a Linux distribution with systemd.

To install Chisel, you can use your distribution's Chisel package or the official install script.
For the sake of this guide, we'll be using the install script:

```bash
curl https://i.jpillora.com/chisel! | bash
```

You'll probably want to make a systemd service to manage the Chisel process.
On the system, you can create a file called `/etc/systemd/system/chisel.service` with the following content:

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
# `foo:bar` is an example of the authentication credentials.
# The format is `username:password`.
# You may also create an auth file with the `--authfile` flag.
ExecStart=/usr/local/bin/chisel server --port=9090 --reverse --auth foo:bar
```

Then run `systemctl daemon-reload` and `systemctl enable --now chisel.service` to enable and start the service. The Chisel server will be accessible on all addresses on port `9000`, although, you may need to configure your firewall settings to allow this.

Now, we can finally let Chisel Operator know about our exit node, by creating a corresponding `ExitNode` resource:

```yaml
apiVersion: chisel-operator.io/v1
kind: ExitNode
metadata:
  name: my-exit-node
  namespace: default
spec:
  # IP address of exit node
  host: "192.168.1.1" # Set to the public IP of your exit node!
  # Control plane socket port
  port: 9090
  # Name of the secret containing the auth key
  # Create a secret with a key named "auth" and put the value there
  auth: my-exit-node-secret
```

We'll also need to create a secret with our credentials:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: my-exit-node-secret
  namespace: default
type: Opaque
stringData:
  auth: user:password
```

And congratulations, you're ready to start tunneling services! That wasn't too hard, was it?
