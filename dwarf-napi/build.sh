#!/bin/bash
# Build the napi-rs native addon
set -e
cd "$(dirname "$0")"
npx @napi-rs/cli build --release
