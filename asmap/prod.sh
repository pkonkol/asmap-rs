#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

pushd frontend
CARGO_TARGET_DIR=../target-trunk trunk build --release --public-url /
#trunk build --release --public-url /
popd

cargo run -p server --release -- --port 8080 --static-dir ./dist --config ../config.yaml
