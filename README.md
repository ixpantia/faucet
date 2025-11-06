# faucet <img src="docs/figures/faucet.png" align="right" width=120 height=139 alt="" />

<!-- badges: start -->
[![Crates.io](https://img.shields.io/crates/v/faucet-server.svg)](https://crates.io/crates/faucet-server)
[![test](https://github.com/ixpantia/faucet/actions/workflows/test.yaml/badge.svg?branch=main)](https://github.com/ixpantia/faucet/actions/workflows/test.yaml)
<!-- badges: end -->

Scale, deploy and route R and Python applications with ease and efficiency.

## Summary

Welcome to faucet, a feature-rich deployment platform for Shiny Applications, Plumber APIs, and FastAPI applications. faucet features load balancing, routing, logging, replication, and more, all in one place, unifying your workflow for deploying R and Python-based applications.

## Features

- **High Performance:** faucet is designed with speed in mind, leveraging Rust's performance benefits to ensure your R and Python applications run smoothly and efficiently.

- **Polyglot Support:** Natively deploy applications written in R (Plumber, Shiny) and Python (FastAPI), or run arbitrary `Rscript` and Python (`uv`) scripts.

- **Load Balancing:** Choose between Round Robin, IP Hash, and Cookie Hash load balancing strategies to distribute incoming requests among multiple instances, optimizing resource utilization.

- **Replicas:** Easily scale your Plumber APIs, Shiny Applications, and FastAPI applications by running multiple replicas, allowing for improved performance and increased availability.

- **Simplified Deployment:** faucet simplifies the deployment process, making it a breeze to get your R and Python applications up and running quickly.

- **Asynchronous & Concurrent:** faucet leverages asynchronous and concurrent processing, ensuring optimal utilization of resources and responsive handling of requests.

- **Routing**: Run multiple Shiny Applications, Plumber APIs, FastAPI applications, and Quarto Documents on a single server with our easy-to-configure router.

## Usage

### Get some help

To display the help message, run the following command:

```bash
faucet --help
```

### Start a Plumber API

To start a plumber API, simply specify the directory containing the `'plumber.R'` file. faucet will automatically detect the file and start the API.

```bash
faucet start --dir /path/to/plumber/api
```

### Start a Shiny Application

To start a Shiny Application, specify the directory containing the `'app.R'` file. faucet will automatically detect the file and start the application.

```bash
faucet start --dir /path/to/shiny/app
```

> **Note:** On Shiny applications, faucet will default to IP Hash load balancing. This is because Shiny applications require a persistent connection between the client and the server.

### Start a FastAPI Application

To start a FastAPI application, specify the directory containing your `main.py` file. faucet uses `uv` to manage the Python environment and run the application.

```bash
faucet start --dir /path/to/fastapi/app --type fast-api
```

### Running Scripts

#### Rscript
Faucet can execute arbitrary R scripts using the `rscript` subcommand. Any arguments following `rscript` are passed directly to the script.

```bash
faucet rscript path/to/your/script.R --arg1 value1
```

#### Python with `uv`
Similarly, you can run any `uv` command, which is useful for executing Python scripts or managing dependencies.

```bash
faucet uv run path/to/your/script.py
```
```bash
faucet uv pip install pandas
```

### Customizing Your Server

The server will listen on port `3838` by default. To change the host and port, use the `--host` flag.

```bash
faucet --host 0.0.0.0:3000 start --dir /path/to/your/app
```

By default, faucet will start as many workers as there are logical cores on the machine. To specify the number of workers, use the `--workers` flag.

```bash
faucet start --dir /path/to/your/app --workers 4
```

### Pick a Load Balancing Strategy

faucet supports multiple load balancing strategies. By default, faucet will use Round Robin for stateless applications (Plumber, FastAPI) and IP Hash for stateful ones (Shiny). To change the strategy, use the `--strategy` flag.

```bash
faucet start --dir /path/to/plumber/api --strategy cookie-hash
```

### Explicitly Set the Type of Application

By default, faucet will try to detect the type of application based on the files in the specified directory. If you want to explicitly set the type of application, use the `--type` flag.

```bash
faucet start --dir /path/to/plumber/api --type plumber
faucet start --dir /path/to/shiny/app --type shiny
faucet start --dir /path/to/fastapi/app --type fast-api
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
flag to either `x-forwarded-for` or `x-real-ip` depending on which header you
set in Nginx.

```bash
faucet --ip-from x-forwarded-for start --dir /path/to/plumber/api
```

## Installation

### Option 1: Binary Download (Linux)

Download the latest release of faucet for Linux from the [GitHub Releases page](https://github.com/ixpantia/faucet/releases).

```bash
# Replace with the desired version
FAUCET_VERSION="v2.1.0"

wget https://github.com/ixpantia/faucet/releases/download/$FAUCET_VERSION/faucet-x86_64-unknown-linux-musl -O faucet
chmod +x faucet
mv faucet ~/.local/bin
```

### Option 2: Install with Cargo (Linux, macOS, Windows)

Install faucet with Cargo, Rust's package manager.

1. Install Rust by following the instructions [here](https://www.rust-lang.org/tools/install).
2. Install faucet with Cargo.

```bash
cargo install faucet-server
```

### Option 3: Build from Source (Linux, macOS, Windows)

1. Install Rust.
2. Clone the faucet repository.
   ```bash
   git clone https://github.com/ixpantia/faucet.git
   cd faucet
   ```
3. Build and install faucet with Cargo.
   ```bash
   cargo install --path .
   ```

## HTTP Telemetry

faucet offers the option of saving HTTP events to a PostgreSQL database.
This can be very helpful for tracking latency, total API calls and other
important information.

In order to use this feature you will need a PostgreSQL database with a table
called `faucet_http_events`. You can create the table with
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

To connect to the database, pass the `FAUCET_TELEMETRY_POSTGRES_STRING` environment variable or the `--pg-con-string` CLI argument. You can also specify a `--telemetry-namespace` to track different services on the same database.

## Useful links

- [faucet Documentation](https://ixpantia.github.io/faucet/)
- [How to Run R Shiny in Docker: A Step-by-Step Guide](https://www.ixpantia.com/en/blog/how-to-run-r-shiny-in-docker-guide)

## Contributing

If you want to contribute to `faucet` please read the
[CONTRIBUTING.md](./CONTRIBUTING.md) document.
