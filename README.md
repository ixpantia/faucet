# faucet <img src="docs/figures/faucet.png" align="right" width=120 height=139 alt="" />

<!-- badges: start -->
[![Crates.io](https://img.shields.io/crates/v/faucet-server.svg)](https://crates.io/crates/faucet-server)
[![test](https://github.com/ixpantia/faucet/actions/workflows/test.yaml/badge.svg?branch=main)](https://github.com/ixpantia/faucet/actions/workflows/test.yaml)
<!-- badges: end -->

Scale, deploy and route Plumber APIs and Shiny applications with ease and efficiency.

## Summary


Welcome to faucet, the most feature complete Shiny Application and Plumber API deployment
platform. faucet features load balancing, routing, logging, replication, and more
all in one place; unifying your workflow for deploying R based applications.

## Features

- **High Performance:** faucet is designed with speed in mind, leveraging Rust's performance benefits to ensure your R applications run smoothly and efficiently.

- **Load Balancing:** Choose between Round Robin and IP Hash load balancing strategies to distribute incoming requests among multiple instances, optimizing resource utilization.

- **Replicas:** Easily scale your Plumber APIs and Shiny Applications by running multiple replicas, allowing for improved performance and increased availability.

- **Simplified Deployment:** faucet simplifies the deployment process, making it a breeze to get your R applications up and running quickly.

- **Asynchronous & Concurrent:** faucet leverages asynchronous and concurrent processing, ensuring optimal utilization of resources and responsive handling of requests.

- **Routing**: Run multiple Shiny Applications, Plumber APIs, Quarto Documents on a single server with our easy to configure router.

## Usage

### Get some help

To display the help message, run the following command:

```bash
faucet --help
```

### Start a Plumber API

To start a plumber API, you will simply need to specify the directory containing the `'plumber.R'` file. faucet will automatically detect the file and start the API.

```bash
faucet start --dir /path/to/plumber/api
```

The server will automatically listen on port `3838` by default. To change the host and port, use the `--host` flag to specify the socket address to bind to the service.

```bash
faucet start --dir /path/to/plumber/api --host 0.0.0.0:3000
```

By default faucet will start as many workers as there are logical cores on the machine. To specify the number of workers, use the `--workers` flag.

```bash
faucet start --dir /path/to/plumber/api --workers 4
```

### Start a Shiny Application

To start a Shiny Application, you will simply need to specify the directory containing the `'app.R'` file. faucet will automatically detect the file and start the application.

```bash
faucet start --dir /path/to/shiny/app
```

The server will automatically listen on port `3838` by default. To change the host and port, use the `--host` flag to specify the socket address to bind to the service.

```bash
faucet start --dir /path/to/shiny/app --host 0.0.0.0:3000
```

By default faucet will start as many workers as there are logical cores on the machine. To specify the number of workers, use the `--workers` flag.

```bash
faucet start --dir /path/to/shiny/app --workers 4
```

> **Note:** On Shiny applications, faucet will be forced to use IP Hash load balancing. This is because Shiny applications require a persistent connection between the client and the server. If Round Robin load balancing is used, the client will be redirected to a different instance on each request, causing the connection to be lost.

### Pick a Load Balancing Strategy for Plumber APIs

faucet supports two load balancing strategies for Plumber APIs: Round Robin and IP Hash.
By default, faucet will use Round Robin load balancing. To change the strategy, use the `--strategy` flag.

```bash
faucet start --dir /path/to/plumber/api --strategy ip-hash
```

### Explicitly Set the Type of Application

By default, faucet will try to detect the type of application based on the files in the specified directory. If you want to explicitly set the type of application, use the `--type` flag.

For Plumber applications:
```bash
faucet start --dir /path/to/plumber/api --type plumber
```

For Shiny applications:
```bash
faucet start --dir /path/to/shiny/app --type shiny
```

For Quarto applications:
```bash
faucet start --qmd /path/to/example.qmd --type quarto-shiny
```

If you are working with a Quarto document, it must be explicitly specified using the `--qmd` flag and the `--type quarto-shiny` option.

## With Nginx / Reverse Proxy

If you want to run multiple faucet instances behind a reverse proxy, or you want to enable HTTPS,
you may use Nginx or any other reverse proxy. However, to make sure faucet correctly detects the
client IP address, you will need to set the `X-Forwarded-For` header or the `X-Real-IP` header.

### Nginx

```nginx
server {
    listen 80;
    server_name example.com;

    location / {
        proxy_pass http://...;
        proxy_set_header  X-Real-IP $remote_addr;
        proxy_set_header  X-Forwarded-For $proxy_add_x_forwarded_for;
        ...
    }
}
```

Additionally, when running faucet, you will need to set the `-i` / `--ip-from`
flat to either `x-forwarded-for` or `x-real-ip` depending on which header you
set in Nginx.

```bash
faucet start --dir /path/to/plumber/api --ip-from x-forwarded-for
```

## Installation

### Option 1: Binary Download (Linux)

Download the latest release of faucet for Linux from the [GitHub Releases page](https://github.com/ixpantia/faucet/releases). This should work with most Linux distributions.

```bash
FAUCET_VERSION="v1.1.0"

wget https://github.com/ixpantia/faucet/releases/download/$FAUCET_VERSION/faucet-x86_64-unknown-linux-musl -O faucet

# Make the binary executable
chmod +x faucet

# Move the binary to a directory in your PATH (e.g., user local bin)
mv faucet ~/.local/bin
```

### Option 2: Install with Cargo (Linux, macOS, Windows)

Install faucet with Cargo, Rust's package manager.

1. Install Rust by following the instructions [here](https://www.rust-lang.org/tools/install).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Install faucet with Cargo.

```bash
cargo install faucet-server --version ^1.1
```

### Option 3: Build from Source (Linux, macOS, Windows)

1. Install Rust by following the instructions [here](https://www.rust-lang.org/tools/install).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Clone the faucet repository.

```bash
git clone https://github.com/ixpantia/faucet.git
```

3. Build faucet with Cargo.

```bash
cargo install --path .
```

## HTTP Telemetry

faucet now offers the option of saving HTTP events to a PostgreSQL database.
This can be very helpful for tracking latency, total API calls and other
important information.

In order to use this feature you will need a PostgreSQL database with a table
called `faucet_http_events`. You can create the table using this table with
the following SQL query:

```sql
CREATE TABLE faucet_http_events (
    request_uuid UUID,
    namespace TEXT,
    version TEXT,
    target TEXT,
    worker_route TEXT,
    worker_id INT,
    ip_addr INET,
    method TEXT,
    path TEXT,
    query_params TEXT,
    http_version TEXT,
    status SMALLINT,
    user_agent TEXT,
    elapsed BIGINT,
    time TIMESTAMPTZ
);
```

### Connection Strings

In order to connect to the database you will need to pass the
`FAUCET_TELEMETRY_POSTGRES_STRING` environment variable or the
`--pg-con-string` CLI argument.

This should include either a connection string or a URL with the `postgres://`
protocol.

#### Example connection strings

```sh
FAUCET_TELEMETRY_POSTGRES_STRING="host=localhost user=postgres connect_timeout=10 keepalives=0"
FAUCET_TELEMETRY_POSTGRES_STRING="host=/var/lib/postgresql,localhost port=1234 user=postgres password='password with spaces'"
FAUCET_TELEMETRY_POSTGRES_STRING="postgresql://user@localhost"
FAUCET_TELEMETRY_POSTGRES_STRING="postgresql://user:password@127.0.0.1/mydb?connect_timeout=10"
```

### Telemetry Namespaces

It is likely you want to track different services on the same database. You
can control the column `namespace` using the environment variable
`FAUCET_TELEMETRY_NAMESPACE` or cli argument `--telemetry-namespace`. By
default, this value is `"faucet"`.

## Contributing

If you want to contribute to `faucet` please read the
[CONTRIBUTING.md](./CONTRIBUTING.md) document.

