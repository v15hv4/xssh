use clap::Parser;
use xssh::{Args, Sync, SSH};

fn main() {
    env_logger::init();

    let args = Args::parse();

    // sync hosts from source
    if args.sync.is_some() {
        let sync = Sync::new(args.overwrite);
        match args.sync.unwrap_or(String::new()).as_str() {
            "tailscale" => sync.tailscale(),
            _ => println!("Invalid sync source!"),
        }
    }

    // spawn SSH connection
    if let Some(destination) = args.destination {
        SSH::new(destination, args.tmux).spawn();
    }
}
