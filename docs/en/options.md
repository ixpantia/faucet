# Options / Config

This section covers all user-configurable options for faucet.

## Global Options

These options can be used with both the `start` and `router` subcommands.

### Host

- CLI: `--host`
- Environment: `FAUCET_HOST`
- Default: `127.0.0.1:3838`

The host and port to bind the faucet server to. If running in a container, this should
be set to `0.0.0.0:3838` to allow external access.

### IP From (How to determine the client IP)

- CLI: `--ip-from` or `-i`
- Environment: `FAUCET_IP_FROM`
- Default: `client`
- Possible values:
  - `client`
  - `x-forwarded-for`
  - `x-real-ip`

How to determine the client IP. This is used to determine the IP for the IP Hash
strategy and for logging of HTTP requests. If you are running faucet directly to end
users, you should use `client`. If you are running faucet behind a reverse proxy like
_nginx_, you should use `x-forwarded-for` or `x-real-ip`.

> **Note:** If you are running faucet behind a reverse proxy, be sure to set the
> `X-Forwarded-For` or `X-Real-IP` header correctly in your reverse proxy\'s
> configuration.

### Rscript (Define a custom `Rscript` binary/executable)

- CLI: `--rscript` or `-r`
- Environment: `FAUCET_RSCRIPT`
- Default: `Rscript`

The `Rscript` binary/executable to use. This is useful if you need to have
multiple versions of R installed on your system. This should be the full path
to the `Rscript` binary/executable or an alias that is available in your
`$PATH`. This is also useful in platforms like _Windows_ where the `Rscript`
binary/executable may not be available in the `$PATH`.

### Quarto (Define a custom `quarto` binary/executable)

- CLI: `--quarto` or `-q`
- Environment: `FAUCET_QUARTO`
- Default: `quarto`

The `quarto` binary/executable to use. This is useful if you have
multiple versions of Quarto installed or if it\'s not in your `$PATH`.

### Uv (Define a custom `uv` binary/executable)

- CLI: `--uv`
- Environment: `FAUCET_UV`
- Default: `uv`

The `uv` binary/executable to use. This is useful if you have multiple versions of `uv` installed, or if it is not in your system's `PATH`. `uv` is required for running FastAPI applications and `uv` subcommands.

### Log File (Redirect logging to a file)

- CLI: `--log-file` or `-l`
- Environment: `FAUCET_LOG_FILE`
- Default: `None`

If you set this variable it will disable colors on `stderr` and save all output
to the specified path. This will append, not overwrite previously existing files.

### Max Log File Size

- CLI: `--max-log-file-size` or `-m`
- Environment: `FAUCET_MAX_LOG_FILE_SIZE`
- Default: `None`

The maximum size of the log file before rotation (e.g., 10M, 1GB).
Requires `log-file` to be set.

### Logging Level

- Environment: `FAUCET_LOG`
- Default: `INFO`
- Possible values:
  - `ERROR`
  - `WARN`
  - `INFO`
  - `DEBUG`
  - `TRACE`

The logging level to use. This environment variable sets the global logging verbosity.
Refer to the [logging](./logging.md) section for more information.
**Note:** While this environment variable is functional, newer applications might prefer
more granular control via dedicated logger configuration files or library-specific
settings if available. The CLI options `--log-file` and `--max-log-file-size`
provide direct control over file-based logging.

### Shutdown

- CLI: `--shutdown`
- Environment: `FAUCET_SHUTDOWN`
- Default: `immediate`
- Possible values:
  - `immediate`
  - `graceful`

The strategy used for shutting down faucet. `immediate` kills every
active connection and shutdown the process. `graceful` waits
for all connections to close before shutting down.

### Max Message Size

- CLI: `--max-message-size`
- Environment: `FAUCEC_MAX_MESSAGE_SIZE`
- Default: `None`

Maximum size of a WebSocket message. This is useful for DDOS prevention.
If not set, there is no size limit.

### Telemetry: PostgreSQL Connection String

- CLI: `--pg-con-string`
- Environment: `FAUCET_TELEMETRY_POSTGRES_STRING`
- Default: `None`

Connection string to a PostgreSQL database for saving HTTP events. If provided,
faucet will attempt to log HTTP events to this database.

### Telemetry: Namespace

- CLI: `--telemetry-namespace`
- Environment: `FAUCET_TELEMETRY_NAMESPACE`
- Default: `faucet`

Namespace under which HTTP events are saved in PostgreSQL.

### Telemetry: Version

- CLI: `--telemetry-version`
- Environment: `FAUCET_TELEMETRY_VERSION`
- Default: `None`

Represents the source code version of the service being run. This is useful for
filtering telemetry data.

### Telemetry: PostgreSQL SSL Certificate

- CLI: `--pg-sslcert`
- Environment: `FAUCET_TELEMETRY_POSTGRES_SSLCERT`
- Default: `None`

Path to a CA certificate file for verifying the PostgreSQL server when using SSL/TLS. Required if `--pg-sslmode` is set to `verify-ca` or `verify-full`. The certificate should be in PEM or DER format.

### Telemetry: PostgreSQL SSL Mode

- CLI: `--pg-sslmode`
- Environment: `FAUCET_TELEMETRY_POSTGRES_SSLMODE`
- Default: `prefer`
- Possible values:
  - `disable`
  - `prefer`
  - `require`
  - `verify-ca`
  - `verify-full`

Controls the SSL/TLS behavior for the PostgreSQL connection. If set to `verify-ca` or `verify-full`, a CA certificate must be provided via `--pg-sslcert` or `FAUCET_TELEMETRY_POSTGRES_SSLCERT`.

## `start` Subcommand Options

These options are specific to the `start` subcommand, used for running a standard faucet server.

### Workers

- CLI: `--workers` or `-w`
- Environment: `FAUCET_WORKERS`
- Default: The number of CPUs available to the process

The number of worker processes to spawn. On a CPU-bound workload, this should be set to
the number of CPUs available to the process. On an IO-bound workload, this could be set
to a higher number.

### Strategy

- CLI: `--strategy` or `-s`
- Environment: `FAUCET_STRATEGY`
- Default: `round-robin`
- Possible values:
  - `round-robin`
  - `ip-hash`
  - `cookie-hash`

The strategy to use for load balancing. Which strategy you choose depends on your
workload.

#### Round Robin

Round robin is a very lightweight and simple load balancing strategy. It simply
distributes requests to workers in a round robin fashion. This can be a good strategy
for most workloads, it is very simple and has very little overhead.

You should **NOT** use round robin if the server is stateful, as it will not guarantee
that requests from the same client will be routed to the same worker. If you need
persistent state, use IP Hash or Cookie Hash.

If a worker dies, the requests that were routed will continue to be the next available
worker that is alive.

#### IP Hash

IP Hash is a more complex strategy that guarantees that requests from the same client
will be routed to the same worker. This is useful for stateful servers, such as Shiny
apps. IP Hash is enforced on Shiny apps if the strategy is set to `auto`.

If a worker dies, the requests will be held until the worker is back online. This means
that latency may increase if a worker dies.

#### Cookie Hash

Cookie Hash uses a cookie to identify the worker to send the request to. This is
useful for sticky sessions from within the same network, even if clients are behind
a NAT or share the same IP address.

### Type (Type of server)

- CLI: `--type` or `-t`
- Environment: `FAUCET_TYPE`
- Default: `auto`
- Possible values:
  - `auto`
  - `plumber`
  - `shiny`
  - `quarto-shiny`
  - `fast-api`

The type of server to run. This is used to determine the correct strategy to use
and how to spawn the workers.

#### Auto

Auto will attempt to determine the type of server based on the contents of the
directory specified by `--dir`.

- If the directory contains a `plumber.R` or `entrypoint.R` file, it will be assumed to be a Plumber server.
- If the directory contains an `app.R`, or both `server.R` and `ui.R` files, it will be assumed to be a Shiny server.
- If a `.qmd` file is provided via the `--qmd` argument, or if `FAUCET_QMD` is set, it will be assumed to be a Quarto Shiny application.
  Otherwise, faucet will exit with an error.

#### Plumber

Runs the server as a Plumber API. The default strategy is `round-robin`.

#### Shiny

Runs the server as a Shiny app. The default strategy is `ip-hash`.

#### Quarto Shiny

Runs the server as a Quarto Shiny app. The default strategy is `ip-hash`.
Requires the `--qmd` option to specify the Quarto document.

#### FastAPI

Runs the server as a FastAPI application. The default strategy is `round-robin`. This requires `uv` to be installed. Faucet will look for a `main.py` file in the specified directory and serve it.

### Directory (Working directory)

- CLI: `--dir` or `-d`
- Environment: `FAUCET_DIR`
- Default: `.`

The directory to run the server from. This should be the directory that contains the
`plumber.R` or Shiny app contents.

### App Directory (Shiny `appDir`)

- CLI: `--app-dir` or `-a`
- Environment: `FAUCET_APP_DIR`
- Default: `None`

Argument passed on to `appDir` when running Shiny applications. This allows you
to specify a subdirectory within the `--dir` path as the root for the Shiny app.

### QMD (Quarto Document)

- CLI: `--qmd`
- Environment: `FAUCET_QMD`
- Default: `None`

Path to the Quarto Shiny `.qmd` file. This is required when `type` is set to
`quarto-shiny`, or when `type` is `auto` and you intend to run a Quarto Shiny app.

## `router` Subcommand Options

These options are specific to the `router` subcommand, used for running faucet in router mode (experimental).

### Config File

- CLI: `--conf` or `-c`
- Environment: `FAUCET_ROUTER_CONF`
- Default: `./frouter.toml`

Path to the router configuration TOML file.

## `rscript` Subcommand

This subcommand allows you to execute an arbitrary R script. Any arguments following `rscript` will be passed directly to the `Rscript` executable.

Example: `faucet rscript my_script.R --arg1 value1`

## `uv` Subcommand

This subcommand allows you to execute arbitrary `uv` commands. This is particularly useful for running Python scripts or managing Python environments. Any arguments following `uv` will be passed directly to the `uv` executable.

Example: `faucet uv run my_script.py` or `faucet uv pip install pandas`
