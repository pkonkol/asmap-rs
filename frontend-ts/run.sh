#!/usr/bin/env sh
set -eu

ROOT_DIR=$(cd "$(dirname "$0")" && pwd)
cd "$ROOT_DIR"

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "wasm-pack is required. Install with: brew install wasm-pack"
  exit 1
fi

npm install
npm run build:wasm
npm run dev
