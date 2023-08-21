# Faucet

Faucet is an asynchronous runtime for running [Plumber](https://www.rplumber.io/) APIs. Faucet enables guaranteed concurrency and parallelism for Plumber APIs without the need for a promise based API. Faucet can run either locally or in a Kubernetes cluster.

> **Note:** Faucet is currently in early development and is not ready for production use.

## Why Faucet?

When building an API with `Plumber` a common pattern is to use [promises](https://rstudio.github.io/promises/index.html), however in many cases promises cause significant overhead by creating new R processes on every request; promises also do not support database connections.

In **local** mode, Faucet uses a lock system (similar to how relational databases handle concurrency) to distribute HTTP requests across multiple `Plumber` workers guaranteeing that a single process only handles one request at a time.

In **K8s** mode, Faucet uses the same lock system to distribute load across a Kubernetes service.

## What makes it different from other async runtimes?

Faucet is similar to other load balancers like [Valve](https://github.com/JosiahParry/valve/) when working on local mode. However, Faucet is designed to be a versatile runtime that can run either in a normal Linux VM or in a Kubernetes cluster. Faucet is able to use it's worker lock on replicas inside a Kubernetes service, avoiding unwanted request collisions.

## Installation

To install Faucet, you first need to install [The Rust Programming Language](https://www.rust-lang.org/) on your system. In the future, platform specific binaries will be distributed.

Clone the repository and run `cargo install --path .` to install the `faucet` binary.

## Usage in local mode

To use Faucet, you will need to have a Plumber API with an entrypoint file named `entrypoint.R` that contains code like the following:

```r
library(plumber)
# 'plumber.R' is the location of the file shown above
pr("plumber.R") %>%
  pr_run(port = as.integer(Sys.getenv("FAUCET_PORT")))
```

The environment variable `FAUCET_PORT` is used to specify the port that the specific `Plumber` worker should listen on. Faucet will automatically set this environment variable for each worker.

To run the API, run the following command while in the same directory as the `entrypoint.R` file:

```bash
faucet
```

If you want to run an API on a different directory, you can use the `--dir` flag:

```bash
faucet local --dir /path/to/api
```

For more information on the available flags, run `faucet local --help`.

## Usage in Kubernetes mode

TODO: Improve documentation

This is a work in progress. Please check back later.

For now you can try a little experiment to understand the basic idea:

1. Run a plumber API locally on port 8000
2. Run `faucet k8s --service-url http://localhost:8000` to start a faucet server
3. Request any resource to the faucet server and see that it is proxied to the plumber API.

What the faucet server will do is resolve the hostname of the `--service-url` flag and acquire a lock on the specific pod hosting the application. In a Kubernetes environment this hostname would dynamically resolve to different IP addresses according to the number of replicas.
