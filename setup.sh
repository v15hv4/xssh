#!/usr/bin/env bash

# determine whether update or first install
UPDATE=false
[ -e /usr/bin/xssh ] && UPDATE=true

# install latest binary
sudo wget https://github.com/v15hv4/xssh/releases/latest/download/xssh -O /usr/bin/xssh
sudo chmod a+x /usr/bin/xssh

# extra setup during first install
if [[ "$UPDATE" = false ]]; then
  # ssh completions
  [[ $SHELL =~ "zsh" ]] && echo -e "\n# xssh completions\ncompdef _ssh xssh=ssh" >> ~/.zshrc
fi
