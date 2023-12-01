# Faucet: Fast, Asynchronous, Concurrent R Application Deployment

Welcome to Faucet, your go-to solution for deploying Plumber APIs and Shiny Applications with blazing speed and efficiency. Faucet is a high-performance server built with Rust, offering Round Robin and Round Robin + IP Hash load balancing for seamless scaling and distribution of your R applications. Whether you're a data scientist, developer, or DevOps enthusiast, Faucet streamlines the deployment process, making it easier than ever to manage replicas and balance loads effectively.

## Features

- **High Performance:** Faucet is designed with speed in mind, leveraging Rust's performance benefits to ensure your R applications run smoothly and efficiently.

- **Load Balancing:** Choose between Round Robin and Round Robin + IP Hash load balancing strategies to distribute incoming requests among multiple instances, optimizing resource utilization.

- **Replicas:** Easily scale your Plumber APIs and Shiny Applications by running multiple replicas, allowing for improved performance and increased availability.

- **Simplified Deployment:** Faucet simplifies the deployment process, making it a breeze to get your R applications up and running quickly.

- **Asynchronous & Concurrent:** Faucet leverages asynchronous and concurrent processing, ensuring optimal utilization of resources and responsive handling of requests.

## Installation

### Option 1: Binary Download (Linux)

Download the latest release of Faucet for Linux from the [GitHub Releases page](https://github.com/yourusername/faucet/releases).

```bash
# Replace "vX.X.X" with the latest version number
$ wget https://github.com/yourusername/faucet/releases/download/vX.X.X/faucet-linux-x86_64 -O faucet

# Make the binary executable
$ chmod +x faucet

# Move the binary to a directory in your PATH (e.g., user local bin)
$ mv faucet ~/.local/bin/
```

### Option 2: Install with Cargo (Linux, macOS, Windows)

Install Faucet with Cargo, Rust's package manager.

1. Install Rust by following the instructions [here](https://www.rust-lang.org/tools/install).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Install Faucet with Cargo.

```bash
cargo install faucet-server
```

### Option 3: Build from Source (Linux, macOS, Windows)

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
