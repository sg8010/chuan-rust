#!/usr/bin/env bash
# 构建本机 GUI；Windows 7 专用交叉构建见 scripts/build-win.sh。
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --locked --bin chuan "$@"
