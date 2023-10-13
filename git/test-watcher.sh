#!/bin/bash

# Watch for changes and run tests on save
cargo watch -c -i *.txt -i test/* -x "test -q --tests"
