#!/usr/bin/env bash

# install binary
sudo wget https://github.com/v15hv4/xssh/releases/latest/download/xssh -O /usr/bin/xssh
sudo chmod a+x /usr/bin/xssh

# ssh completions for zsh
echo "compdef _ssh xssh=ssh" > ~/.zshrc
