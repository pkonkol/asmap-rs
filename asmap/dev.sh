#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

(trap 'kill 0' SIGINT; \
    bash -c 'cd frontend; CARGO_TARGET_DIR=../target-trunk trunk serve --address 0.0.0.0 --port 8079' & \
    bash -c 'cd server; cargo watch -- cargo run -- --port 8080 --config ../../config.yaml')
