pub fn generate_cloud_init_config(auth_string: &str, port: u16) -> String {
    let cloud_config = serde_json::json!({
      "runcmd": ["curl https://i.jpillora.com/chisel! | bash", "systemctl enable --now chisel"],
      "write_files": [{
        "path": "/etc/systemd/system/chisel.service",
        "content": format!(r#"
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
ExecStart=/usr/local/bin/chisel server --port={port} --reverse --auth {auth_string}
# Additional .env file for auth and secrets
EnvironmentFile=-/etc/sysconfig/chisel
PassEnvironment=AUTH
"#)
      }, {
        "path": "/etc/sysconfig/chisel",
        "content": format!("AUTH={auth_string}\n")
      }]
    });

    "#cloud-config\n".to_string() + &cloud_config.to_string()
}

#[test]
fn test_generate_cloud_init_config() {
    let password = "chisel:test";
    let config = generate_cloud_init_config(password, 9090);
    println!("{}", config);
    assert!(config.contains("AUTH=chisel:test"));
    assert!(config.contains("ExecStart=/usr/local/bin/chisel server --port=9090 --reverse --auth chisel:test"));
}
