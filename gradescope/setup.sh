#!/usr/bin/env bash

sudo apt update

curl -sL https://deb.nodesource.com/setup_8.9 -o nodesource_setup.sh
sudo bash nodesource_setup.sh
sudo apt install -y python3 jq build-essential nodejs

git clone https://github.com/mxheller/cs19-autograding.git --recurse-submodules
cd cs19-autograding/powder-monkey/pyret-lang
npm install
make
