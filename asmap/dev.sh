#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

(trap 'kill 0' SIGINT; \
    bash -c 'cd frontend-ts; npm run dev -- --host 0.0.0.0 --port 5173' & \
    bash -c 'cd server; cargo watch -- cargo run -- --port 8080 --config ../../config.yaml')
