use swish::{construct_ssh_args, spawn_ssh, Args};

use clap::Parser;

fn main() {
    let args = Args::parse();

    if !args.sync.is_empty() {
    } else if !args.destination.is_empty() {
        let ssh_args = construct_ssh_args(args.destination, args.tmux);
        spawn_ssh(ssh_args);
    } else {
        println!("Nothing to do.");
    }
}
