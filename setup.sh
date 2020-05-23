#!/usr/bin/env bash

apt update
cd /autograder

# Install node and npm
curl -sL https://deb.nodesource.com/setup_8.9 -o nodesource_setup.sh
bash nodesource_setup.sh
apt install -y make python3 jq build-essential nodejs npm unzip

# Clone result processing repo
npm install -g typescript ts-node @types/node
git clone https://github.com/tdelv/result-processing.git

# Unpack pyret zip
unzip "source/autograder/pyret-lang.zip" -d pyret-lang
