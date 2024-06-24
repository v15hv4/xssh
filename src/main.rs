// use xssh::{construct_ssh_args, spawn_ssh, sync_tailscale, Args};

use clap::Parser;
use xssh::{Args, Sync, SSH};

fn main() {
    let args = Args::parse();

    // sync hosts from source
    if !args.sync.is_empty() {
        let sync = Sync::new(args.overwrite);
        match args.sync.as_str() {
            "tailscale" => sync.tailscale(),
            _ => println!("Invalid sync source!"),
        }
    }

    // spawn SSH connection
    if let Some(destination) = args.destination {
        SSH::new(destination, args.tmux).spawn();
    }
}
