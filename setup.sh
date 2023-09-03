#!/usr/bin/env bash

apt update
cd /autograder

# Install node and npm
curl -sL https://deb.nodesource.com/setup_20.x | bash -
apt-get install -y nodejs
apt install -y make python3 jq build-essential nodejs npm unzip

# Clone result processing repo
npm install -g typescript ts-node
git clone https://github.com/brown-cs19/result-processing.git

# Unpack pyret zip
unzip "source/autograder/pyret-lang.zip" -d pyret-lang
