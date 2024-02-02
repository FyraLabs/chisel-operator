---
title: Self-hosting an Exit Node
description: A guide of how you can self-host your own Chisel exit node.
---

First, you'll need a machine where you can run Chisel.
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
ExecStart=/usr/local/bin/chisel server --port=9090 --reverse
# Additional .env file for auth and secrets
EnvironmentFile=-/etc/sysconfig/chisel
```

You'll also need to setup authentication for your Chisel instance. For the above systemd service, this is done in the `/etc/sysconfig/chisel` file:

```bash
# This is the root credentials for the Chisel server. You can change this to whatever you want. Just make sure to keep it a secret.
# You can also use the `--authfile` argument in the ExecStart command instead of this, for a custom ACL file (in JSON).
AUTH=user:password
```

Then run `systemctl daemon-reload` and `systemctl enable --now chisel.service` to enable and start the service. The Chisel server will be accessible on all addresses on port `9000`, although, you may need to configure your firewall settings to allow this.

Congratulations, that wasn't too hard, was it?
