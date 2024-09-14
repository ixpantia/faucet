# Options / Config

This section covers all user-configurable options for faucet.

## Host

- CLI: `--host`
- Environment: `FAUCET_HOST`
- Default: `127.0.0.1:3838`

The host and port to bind the faucet server to. If running in a container, this should
be set to `0.0.0.0:3838` to allow external access.

## Workers

- CLI: `--workers` or `-w`
- Environment: `FAUCET_WORKERS`
- Default: The number of CPUs available to the process

The number of worker processes to spawn. On a CPU-bound workload, this should be set to
the number of CPUs available to the process. On an IO-bound workload, this could be set
to a higher number.

## Strategy

- CLI: `--strategy` or `-s`
- Environment: `FAUCET_STRATEGY`
- Default:
    - Plumber: `round-robin`
    - Shiny: `ip-hash`
- Posibble values:
    - `round-robin`
    - `ip-hash`

The strategy to use for load balancing. Which strategy you choose depends on your
workload.



### Round Robin

Round robin is a very lightweight and simple load balancing strategy. It simply
distributes requests to workers in a round robin fashion. This can be a good strategy
for most workloads, it is very simple and has very little overhead.

You should **NOT** use round robin if the server is stateful, as it will not guarantee
that requests from the same client will be routed to the same worker. If you need
persistent state, use IP Hash.

If a worker dies, the requests that were routed will continue to be the next available
worker that is alive.

### IP Hash

IP Hash is a more complex strategy that guarantees that requests from the same client
will be routed to the same worker. This is useful for stateful servers, such as Shiny
apps.

IP Hash is enforced on Shiny apps, as round robin simply will not work with them.

If a worker dies, the requests will be held until the worker is back online. This means
that latency may increase if a worker dies.

## Type (Type of server)

- CLI: `--type` or `-t`
- Environment: `FAUCET_TYPE`
- Default: `auto`
- Possible values:
    - `auto`
    - `plumber`
    - `shiny`

The type of server to run. This is used to determine the correct strategy to use
and how to spawn the workers.

### Auto

Auto will attempt to determine the type of server based on the contents of the
directory. If the directory contains a `plumber.R` or `entrypoint.R` file, it
will be assumed to be a plumber server. If the directory contains a `app.R`, or
a `server.R` and `ui.R`, it will be assumed to be a Shiny server.

### Shiny

Shiny will run the server as a Shiny app. This will use the IP Hash strategy.

### Plumber

Plumber will run the server as a Plumber server. This will use the Round Robin strategy
unless the `--strategy` option is set to `ip-hash`.

## Directory (Working directory)

- CLI: `--dir` or `-d`
- Environment: `FAUCET_DIR`
- Default: `.`

The directory to run the server from. This should be the directory that contains the
`plumber.R` or Shiny app contents.

## IP From (How to determine the client IP)

- CLI: `--ip-from`
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
> `X-Forwarded-For` or `X-Real-IP` header correctly in your reverse proxy's
> configuration.

## Logging

- Environment: `FAUCET_LOG`
- Default: `INFO`
- Possible values:
    - `ERROR`
    - `WARN`
    - `INFO`
    - `DEBUG`
    - `TRACE`

The logging level to use. Refer to the [logging](./logging.md) section for more
information.

## Redirect logging to a file

- CLI: `--log-file`
- Environment: `FAUCET_LOG_FILE`
- Default: `None`

If you set this variable it will disable colors on `stderr` and save all output
to the specified path. This will append, not overwrite previously existing files.

## Define a custom `Rscript` binary/executable

- CLI: `--rscript` or `-r`
- Environment: `FAUCET_RSCRIPT`
- Default: `Rscript`

The `Rscript` binary/executable to use. This is useful if you need to have
multiple versions of R installed on your system. This should be the full path
to the `Rscript` binary/executable or an alias that is available in your
`$PATH`. This is also useful in platforms like _Windows_ where the `Rscript`
binary/executable may not be available in the `$PATH`.
