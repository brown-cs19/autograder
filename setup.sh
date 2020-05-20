#!/usr/bin/env bash

apt update

curl -sL https://deb.nodesource.com/setup_8.9 -o nodesource_setup.sh
bash nodesource_setup.sh
apt install -y make python3 jq build-essential nodejs npm unzip

unzip "/autograder/source/pyret-lang.zip" -d /autograder/pyret-lang
