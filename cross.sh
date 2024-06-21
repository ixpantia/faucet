#!/bin/bash

# This is a script to cross-compile the project for Windows and Linux
mkdir -p target/cross

rustup target add aarch64-unknown-linux-musl
rustup target add x86_64-unknown-linux-musl
rustup target add x86_64-pc-windows-gnu

function build() {
    export TARGET=$1
    export EXTENSION=$2
    echo "Building for $TARGET"
    cross build --release --target $TARGET
    mv $CARGO_TARGET_DIR/$TARGET/release/faucet$EXTENSION target/cross/faucet-$TARGET$EXTENSION
    sha256sum target/cross/faucet-$TARGET$EXTENSION > target/cross/faucet-$TARGET$EXTENSION.sha256
}

# Build for Linux 
build aarch64-unknown-linux-musl
build x86_64-unknown-linux-musl

# Build for Windows
build x86_64-pc-windows-gnu .exe
