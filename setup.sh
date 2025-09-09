#!/usr/bin/env bash

apt update
cd /autograder

apt-get install -y curl ca-certificates gnupg xz-utils unzip git make python3 jq build-essential

# Install modern Node (20 LTS)
curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
apt-get install -y nodejs

# Useful global tools for TS
npm install -g typescript@5 ts-node@10

# Sanity check
echo "Node:" "$(node -v)"
echo "NPM:"  "$(npm -v)"


# Clone result processing repo
npm install -g typescript@5.0 ts-node
git clone https://github.com/brown-cs19/result-processing.git

# Unpack pyret zip
unzip "source/autograder/pyret-lang.zip" -d pyret-lang
