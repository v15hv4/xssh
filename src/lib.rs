use std::io::Write;

use clap::Parser;
use serde::Deserialize;

use std::{collections::HashMap, fs::OpenOptions, process::Command};

#[derive(Parser)]
#[command(version, arg_required_else_help(true))]
pub struct Args {
    #[arg(index = 1)]
    pub destination: Option<String>,

    #[arg(long, short, default_value_t = String::new())]
    pub tmux: String,

    #[arg(long, default_value_t = false)]
    pub save: bool,

    #[arg(long, default_value_t = String::new(), conflicts_with = "destination")]
    pub sync: String,
}

pub fn construct_ssh_args(destination: String, tmux: String) -> Vec<String> {
    let mut ssh_args = vec![destination];

    // spawn into tmux session on the remote destination
    if !tmux.is_empty() {
        format!("-t tmux -u new -As{}", tmux)
            .split(" ")
            .map(|i| i.to_string())
            .for_each(|i| ssh_args.push(i));
    }

    ssh_args
}

pub fn spawn_ssh(args: Vec<String>) {
    Command::new("ssh")
        .args(args)
        .spawn()
        .expect("Failed to spawn ssh process!")
        .wait_with_output()
        .expect("Exited.");
}

pub fn infer_user(ip: &str) -> String {
    for user in ["ubuntu", "debian", "root"] {
        let ssh_args = format!("-o StrictHostKeyChecking=no -o ConnectTimeout=5 -o BatchMode=true -Cq {user}@{ip} exit").split(" ").map(|i| i.to_string()).collect::<Vec<String>>();
        let status = Command::new("ssh")
            .args(ssh_args)
            .status()
            .expect("Failed to execute ssh command!");

        if status.code().unwrap() == 0 {
            return user.to_string();
        }
    }

    "root".to_string()
}

pub fn sync_tailscale() {
    let tailscale_output = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .unwrap();

    let tailscale_jsonstr = String::from_utf8(tailscale_output.stdout).unwrap();
    let tailscale_json: serde_json::Value =
        serde_json::from_str(&tailscale_jsonstr).expect("Improperly formatted JSON!");

    #[derive(Deserialize, Debug)]
    struct TailscalePeer {
        #[serde(rename = "HostName")]
        hostname: String,
        #[serde(rename = "TailscaleIPs")]
        ips: Vec<String>,
        #[serde(rename = "Tags")]
        tags: Option<Vec<String>>,
    }

    // fetch peer list
    let peers: HashMap<String, TailscalePeer> =
        serde_json::from_value(tailscale_json.get("Peer").unwrap().to_owned()).unwrap();

    // map raw peer list to required host format
    #[derive(Debug, Clone)]
    struct Host {
        user: String,
        hostname: String,
        ip: String,
    }

    // TODO: parallellize
    let peers: Vec<Host> = peers
        .into_iter()
        .map(|(_, v)| v)
        .filter(|v| {
            v.tags
                .clone()
                .is_some_and(|v| v.contains(&"tag:server".to_string()))
        })
        .map(|v| {
            let host = Host {
                user: infer_user(&v.ips[0]),
                hostname: v.hostname,
                ip: v.ips[0].clone(),
            };
            dbg!(&host);
            host
        })
        .collect();

    // TODO: implement struct for SSHConfig with read, write, update methods
    // TODO: implement function to parse & fetch current config file
    let mut hosts: HashMap<String, Host> = HashMap::new();

    // update hosts file with peers
    for peer in peers {
        if hosts.contains_key(&peer.hostname) {
            // TODO: do something for confirmation/overwrite
        }
        hosts.insert(peer.hostname.clone(), peer);
    }

    // write hosts file
    // TODO: move to SSHConfig struct
    {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open("/home/v15hv4/.ssh/config")
            .unwrap();

        for (hostname, host) in &hosts {
            let hoststr = format!(
                r#"
Host {}
    HostName {}
    User {}

            "#,
                hostname, host.ip, host.user
            );

            if let Err(e) = write!(file, "{}", hoststr.trim()) {
                eprintln!("Error writing {hostname} to file:  {}", e)
            }
        }
    }
}
