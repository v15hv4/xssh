use clap::Parser;

use std::process::Command;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Args {
    #[arg(index = 1)]
    pub destination: String,

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
