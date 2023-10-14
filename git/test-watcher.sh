#!/bin/bash

# Watch for changes and run tests on save
cargo watch -c -i *.txt -i tests/data -x "test -q --tests"
