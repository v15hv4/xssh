use swish::{construct_ssh_args, spawn_ssh, sync_tailscale, Args};

use clap::Parser;

fn main() {
    let args = Args::parse();

    if !args.sync.is_empty() {
        match args.sync.as_str() {
            "tailscale" => sync_tailscale(),
            _ => println!("Invalid sync source!"),
        }
    } else if let Some(destination) = args.destination {
        let ssh_args = construct_ssh_args(destination, args.tmux);
        spawn_ssh(ssh_args);
    } else {
        println!("Nothing to do.");
    }
}
