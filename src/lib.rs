mod constants;

use clap::Parser;
use serde::Deserialize;

use std::io::Write;
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

    #[arg(long, default_value_t = false)]
    pub overwrite: bool,
}

#[derive(Debug)]
pub struct SSH {
    args: Vec<String>,
}
impl SSH {
    pub fn new(destination: String, tmux: String) -> Self {
        let mut args = vec![destination];

        // spawn into tmux session on the remote destination
        if !tmux.is_empty() {
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

#[derive(Debug)]
pub struct SSHHost {
    pub hostname: String,
    pub user: String,
    pub ip: String,
}
impl SSHHost {
    pub fn new(hostname: String, ip: String, user: Option<String>) -> Self {
        // infer username if not provided
        if user.is_none() {
            return Self {
                hostname,
                user: Self::infer_user(&ip).to_string(),
                ip,
            };
        }

        Self {
            hostname,
            user: user.unwrap(),
            ip,
        }
    }

    pub fn infer_user(ip: &str) -> &str {
        for user in constants::SSH_USERS {
            let ssh_args = format!("-o StrictHostKeyChecking=no -o ConnectTimeout=5 -o BatchMode=true -Cq {user}@{ip} exit").split(" ").map(|i| i.to_string()).collect::<Vec<String>>();
            let status = Command::new("ssh")
                .args(ssh_args)
                .status()
                .expect("Failed to execute ssh command!");
            if status.code().unwrap() == 0 {
                return user;
            }
        }

        constants::SSH_USERS.last().unwrap()
    }
}
impl ToString for SSHHost {
    fn to_string(&self) -> String {
        format!(
            r#"
Host {}
    HostName {}
    User {}

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
    pub fn load(filename: String) -> Self {
        // TODO: read config file

        Self {
            filename,
            hosts: HashMap::new(),
        }
    }

    pub fn save(&self) {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(&self.filename)
            .unwrap();

        for (hostname, host) in &self.hosts {
            if let Err(e) = write!(file, "{}", host.to_string().trim()) {
                eprintln!("Error writing {hostname} to file:  {}", e)
            }
        }
    }

    pub fn add(&mut self, host: SSHHost, overwrite: bool) {
        // do nothing if the hostname already exists and
        // the user doesn't want to overwrite
        if self.hosts.contains_key(&host.hostname) && !overwrite {
            println!("'{}' exists in the config, skipping...", host.hostname);
            return;
        }

        self.hosts.insert(host.hostname.clone(), host);
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
        let tailscale_output = Command::new("tailscale")
            .args(["status", "--json"])
            .output()
            .unwrap();

        let tailscale_jsonstr = String::from_utf8(tailscale_output.stdout).unwrap();
        let tailscale_json: serde_json::Value =
            serde_json::from_str(&tailscale_jsonstr).expect("Improperly formatted JSON!");

        // fetch peer list
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

    pub fn tailscale(&self) {
        let peers: Vec<TailscalePeer> = Tailscale::new().peers;

        // TODO: parallelize
        let hosts: Vec<SSHHost> = peers
            .into_iter()
            .map(|p| SSHHost::new(p.hostname, p.ips[0].clone(), None))
            .collect();

        let mut config = SSHConfig::load(constants::SSH_CONFIG_FILE.to_string());
        for host in hosts {
            config.add(host, self.overwrite);
        }
        config.save();
    }
}
