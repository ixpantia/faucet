# Faucet: Fast, Asynchronous, Concurrent R Application Deployment

Welcome to Faucet, your go-to solution for deploying Plumber APIs and Shiny Applications with blazing speed and efficiency. Faucet is a high-performance server built with Rust, offering Round Robin and Round Robin + IP Hash load balancing for seamless scaling and distribution of your R applications. Whether you're a data scientist, developer, or DevOps enthusiast, Faucet streamlines the deployment process, making it easier than ever to manage replicas and balance loads effectively.

## Features

- **High Performance:** Faucet is designed with speed in mind, leveraging Rust's performance benefits to ensure your R applications run smoothly and efficiently.

- **Load Balancing:** Choose between Round Robin and Round Robin + IP Hash load balancing strategies to distribute incoming requests among multiple instances, optimizing resource utilization.

- **Replicas:** Easily scale your Plumber APIs and Shiny Applications by running multiple replicas, allowing for improved performance and increased availability.

- **Simplified Deployment:** Faucet simplifies the deployment process, making it a breeze to get your R applications up and running quickly.

- **Asynchronous & Concurrent:** Faucet leverages asynchronous and concurrent processing, ensuring optimal utilization of resources and responsive handling of requests.

## Usage

### Get some help

To display the help message, run the following command:

```bash
faucet --help
```

### Start a Plumber API

To start a plumber API, you will simply need to specify the directory containing the `'plumber.R'` file. Faucet will automatically detect the file and start the API.

```bash
faucet --dir /path/to/plumber/api
```

The server will automatically listen on port `3838` by default. To change the host and port, use the `--host` flag to specify the socket address to bind to the service.

```bash
faucet --dir /path/to/plumber/api --host 0.0.0.0:3000
```

By default Faucet will start as many workers as there are logical cores on the machine. To specify the number of workers, use the `--workers` flag.

```bash
faucet --dir /path/to/plumber/api --workers 4
```

### Start a Shiny Application

To start a Shiny Application, you will simply need to specify the directory containing the `'app.R'` file. Faucet will automatically detect the file and start the application.

```bash
faucet --dir /path/to/shiny/app
```

The server will automatically listen on port `3838` by default. To change the host and port, use the `--host` flag to specify the socket address to bind to the service.

```bash
faucet --dir /path/to/shiny/app --host 0.0.0.0:3000
```

By default Faucet will start as many workers as there are logical cores on the machine. To specify the number of workers, use the `--workers` flag.

```bash
faucet --dir /path/to/shiny/app --workers 4
```

> **Note:** On Shiny applications, Faucet will be forced to use Round Robin + IP Hash load balancing. This is because Shiny applications require a persistent connection between the client and the server. If Round Robin load balancing is used, the client will be redirected to a different instance on each request, causing the connection to be lost.

### Pick a Load Balancing Strategy for Plumber APIs

Faucet supports two load balancing strategies for Plumber APIs: Round Robin and Round Robin + IP Hash.
By default, Faucet will use Round Robin load balancing. To change the strategy, use the `--strategy` flag.

```bash
faucet --dir /path/to/plumber/api --strategy round-robin-ip-hash
```

### Explicitly set the type of application

By default, Faucet will try to detect the type of application based on the files in the specified directory. If you want to explicitly set the type of application, use the `--type` flag.

```bash
faucet --dir /path/to/plumber/api --type plumber
```

```bash
faucet --dir /path/to/shiny/app --type shiny
```

## Installation

### Option 1: Binary Download (Linux)

Download the latest release of Faucet for Linux from the [GitHub Releases page](https://github.com/andyquinterom/faucet/releases). This should work with most Linux distributions.

```bash
FAUCET_VERSION="v0.2.3"

wget https://github.com/andyquinterom/Faucet/releases/download/$FAUCET_VERSION/faucet-x86_64-unknown-linux-musl -O faucet

# Make the binary executable
chmod +x faucet

# Move the binary to a directory in your PATH (e.g., user local bin)
mv faucet ~/.local/bin
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
