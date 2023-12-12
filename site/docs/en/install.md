# Installation

## Option 1: Binary Download (Linux)

Download the latest release of Faucet for Linux from the
[GitHub Releases page](https://github.com/andyquinterom/faucet/releases).

```bash
FAUCET_VERSION="v0.3.1"

wget https://github.com/andyquinterom/Faucet/releases/download/$FAUCET_VERSION/faucet-x86_64-unknown-linux-musl -O faucet

# Make the binary executable
chmod +x faucet

# Move the binary to a directory in your PATH (e.g., user local bin)
mv faucet ~/.local/bin
```

> **Note:**
> While the binary download is expected to work on most Linux distributions,
> compatibility is not guaranteed for all systems. If you encounter issues,
> consider using the Cargo installation or building from source options.

## Option 2: Install with Cargo (Linux, macOS, Windows)

Install Faucet with Cargo, Rust's package manager.

1. Install Rust by following the instructions [here](https://www.rust-lang.org/tools/install).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Install Faucet with Cargo.

```bash
cargo install faucet-server --version 0.3.1
```

## Option 3: Build from Source (Linux, macOS, Windows)

1. Install Rust by following the instructions [here](https://www.rust-lang.org/tools/install).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Clone the Faucet repository.

```bash
git clone https://github.com/andyquinterom/Faucet.git
```

3. Build Faucet with Cargo.

```bash
cargo install --path .
```
