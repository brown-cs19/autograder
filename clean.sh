#!/bin/bash

TRASH="$(mktemp -d -p .)"
mv ./result $TRASH
rm -rf $TRASH &
rm -f results.csv results.json stdout.txt problems
