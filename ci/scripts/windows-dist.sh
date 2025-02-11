#!/usr/bin/env bash

set -e

dist_for() {
    target=$1
    name=$2

    echo "Caching offline packages for $target"
    cargo dev vendor --target $target --name $name

    echo "Generating dist package for $target"
    cargo dev dist --target $target --name $name
}

# 安装 Rust
sh ci/scripts/install-rust.sh

# 安装 nodejs 18.x
sh ci/scripts/install-nodejs.sh

# 安装 pnpm
npm set strict-ssl false && npm install -g pnpm

# 安装 Tauri CLI
# cargo install tauri-cli@1.6.4
sh ci/scripts/install-tauri-cli.sh

if [[ -z "$HOST_TRIPLE" ]]; then
    echo "no target triple specified"
    exit 1
fi

dist_for $HOST_TRIPLE community

# dist for GNU as well if running on `msvc` abi
if [[ $HOST_TRIPLE == *"windows-msvc"* ]]; then
    gnu=$(echo "$HOST_TRIPLE" | sed 's/msvc$/gnu/')
    dist_for $gnu community
fi
