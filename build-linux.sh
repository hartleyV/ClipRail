#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release
cp target/release/cliprail ./ClipRail
strip ./ClipRail 2>/dev/null || true
printf 'Built: %s/ClipRail\n' "$PWD"
