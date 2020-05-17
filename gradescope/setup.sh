#!/usr/bin/env bash

apt update

curl -sL https://deb.nodesource.com/setup_8.9 -o nodesource_setup.sh
bash nodesource_setup.sh
apt install -y make python3 jq build-essential nodejs npm

git clone https://github.com/mxheller/cs19-autograding.git --recurse-submodules
cd cs19-autograding/powder-monkey/pyret-lang
npm install
make
