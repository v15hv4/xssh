mod constants;

use rayon::prelude::*;

use clap::Parser;
use expanduser::expanduser;
use log::{error, info, warn};
use serde::Deserialize;

use std::io::Write;
use std::{collections::HashMap, fs::OpenOptions, process::Command};

#[derive(Parser)]
#[command(version, arg_required_else_help(true))]
pub struct Args {
    #[arg(index = 1)]
    pub destination: Option<String>,

    #[arg(long, short)]
    pub tmux: Option<String>,

    #[arg(long, default_value_t = false)]
    pub save: bool,

    #[arg(long, conflicts_with = "destination")]
    pub sync: Option<String>,

    #[arg(long, default_value_t = false)]
    pub overwrite: bool,
}

#[derive(Debug)]
pub struct SSH {
    args: Vec<String>,
}
impl SSH {
    pub fn new(destination: String, tmux: Option<String>) -> Self {
        let mut args = vec![destination];

        // spawn into tmux session on the remote destination
        if let Some(tmux) = tmux {
            format!("-t tmux -u new -As{}", tmux)
                .split(" ")
                .map(|i| i.to_string())
                .for_each(|i| args.push(i));
        }

        Self { args }
    }

    pub fn spawn(&self) {
        Command::new("ssh")
            .args(&self.args)
            .spawn()
            .expect("Failed to spawn ssh process!")
            .wait_with_output()
            .expect("Exited.");
    }
}

#[derive(Debug, Clone)]
pub struct SSHHost {
    pub hostname: String,
    pub user: String,
    pub ip: String,
}
impl SSHHost {
    pub fn new(hostname: String, ip: String, user: Option<String>) -> Self {
        // detect username if not provided
        if user.is_none() {
            info!("Inferring user for '{hostname}' ({ip})...");
            let user = Self::infer_user(&ip).to_string();
            return Self { hostname, user, ip };
        }

        Self {
            hostname,
            user: user.unwrap(),
            ip,
        }
    }

    pub fn infer_user(ip: &str) -> &str {
        let default = constants::SSH_USERS.last().unwrap();

        for user in constants::SSH_USERS {
            let timeout_args: Vec<String> =
                format!("-k {} {}", constants::WAIT_KILL, constants::WAIT_TERM)
                    .split(" ")
                    .map(|i| i.to_string())
                    .collect();
            let ssh_args: Vec<String> = 
                format!("ssh -o StrictHostKeyChecking=no -o ConnectTimeout=5 -o BatchMode=true -Cq {}@{} exit", user, ip)
                    .split(" ")
                    .map(|i| i.to_string())
                    .collect();
            let cmd_args: Vec<String> = timeout_args.into_iter().chain(ssh_args.into_iter()).collect();
            let status = Command::new("timeout")
                .args(cmd_args)
                .status()
                .expect("Failed to execute ssh command!");
            if status.code().unwrap() == 0 {
                info!("Detected username: {user}");
                return user;
            }
        }

        info!("Unable to detect username, defaulting to: {default}");
        default
    }
}
impl ToString for SSHHost {
    fn to_string(&self) -> String {
        format!(r#"
Host {}
    HostName {}
    User {}
    StrictHostKeyChecking no
"#,
            self.hostname, self.ip, self.user
        )
    }
}

#[derive(Debug)]
pub struct SSHConfig {
    filename: String,
    pub hosts: HashMap<String, SSHHost>,
}
impl SSHConfig {
    // load existing ssh config file
    pub fn load(filename: String) -> Self {
        // TODO: read config file
        let hosts = HashMap::new();
        info!("Loaded config file with {} hosts.", hosts.len());

        Self { filename, hosts }
    }

    // save current config to file
    pub fn save(&self) {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(expanduser(&self.filename).unwrap())
            .unwrap();

        for (hostname, host) in &self.hosts {
            if let Err(e) = write!(file, "{}", host.to_string()) {
                error!("Error writing {hostname} to file:  {}", e)
            }
        }

        info!("Saved hosts to config file.");
    }

    // add host to config
    pub fn add(&mut self, host: SSHHost, overwrite: bool) {
        // do nothing if the hostname already exists and
        // the user doesn't want to overwrite
        if self.hosts.contains_key(&host.hostname) && !overwrite {
            warn!("'{}' exists in the config, skipping...", host.hostname);
            return;
        }

        self.hosts.insert(host.hostname.clone(), host);

        info!("Added host to config.");
    }

    // list hostnames in config
    pub fn list(&self) -> Vec<String> {
        self.hosts.clone().into_keys().collect()
    }
}

#[derive(Deserialize, Debug)]
pub struct TailscalePeer {
    #[serde(rename = "HostName")]
    pub hostname: String,
    #[serde(rename = "TailscaleIPs")]
    pub ips: Vec<String>,
    #[serde(rename = "Tags")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct Tailscale {
    pub peers: Vec<TailscalePeer>,
}
impl Tailscale {
    pub fn new() -> Self {
        // parse tailscale CLI output
        let tailscale_output = Command::new("tailscale")
            .args(["status", "--json"])
            .output()
            .unwrap();
        let tailscale_jsonstr = String::from_utf8(tailscale_output.stdout).unwrap();
        let tailscale_json: serde_json::Value =
            serde_json::from_str(&tailscale_jsonstr).expect("Improperly formatted JSON!");

        // fetch servers from peer list
        let peers: HashMap<String, TailscalePeer> =
            serde_json::from_value(tailscale_json.get("Peer").unwrap().to_owned()).unwrap();
        let peers: Vec<TailscalePeer> = peers
            .into_iter()
            .map(|(_, v)| v)
            .filter(|v| {
                v.tags
                    .clone()
                    .is_some_and(|v| v.contains(&"tag:server".to_string()))
            })
            .collect();

        info!("Loaded {} peers from Tailscale.", peers.len());

        Self { peers }
    }
}

#[derive(Debug)]
pub struct Sync {
    overwrite: bool,
}
impl Sync {
    pub fn new(overwrite: bool) -> Self {
        Self { overwrite }
    }

    // sync hosts from tailscale
    pub fn tailscale(&self) {
        info!("Syncing hosts from Tailscale...");

        let peers: Vec<TailscalePeer> = Tailscale::new().peers;

        // map peers to hosts
        let hosts: Vec<SSHHost> = peers
            .par_iter()
            .map(|p| SSHHost::new(p.hostname.clone(), p.ips[0].clone(), None))
            .collect();

        let mut config = SSHConfig::load(constants::SSH_CONFIG_FILE.to_string());
        for host in hosts {
            config.add(host, self.overwrite);
        }

        config.save();
    }
}
