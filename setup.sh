#!/usr/bin/env bash

apt update
cd /autograder

# Install node and npm
curl -sL https://deb.nodesource.com/setup_8.9 -o nodesource_setup.sh
bash nodesource_setup.sh
apt install -y make python3 jq build-essential nodejs npm unzip

curl -fsSL https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
source ~/.bashrc
nvm install 24
nvm use 24
ln -sf "$(command -v node)" /usr/bin/nodejs #symlink to node

# Clone result processing repo
npm install -g typescript@5.0 ts-node
git clone https://github.com/brown-cs19/result-processing.git

# Unpack pyret zip
unzip "source/autograder/pyret-lang.zip" -d pyret-lang
