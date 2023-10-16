#!/bin/bash

# Watch for changes and run tests on save
cargo watch -c -i *.txt -i tests/data -i test-watcher.sh -i clippy-watcher.sh -x "clippy -- -W clippy::pedantic"
