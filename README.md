# Faucet

Faucet is an asynchronous runtime for running [Plumber](https://www.rplumber.io/) APIs. Faucet enabled guaranteed concurrency and parallelism for Plumber APIs without the need for a promise based API.

> **Note:** Faucet is currently in early development and is not ready for production use.

## Why Faucet?

When building an API with `Plumber` a common pattern is to use [promises](https://rstudio.github.io/promises/index.html), however in many cases promises cause significant overhead by creating new R processes on every request; promises also do not support database connections.

Faucet uses a lock system (similar to how relational databases handle concurrency) to distribute HTTP requests across multiple `Plumber` workers guaranteeing that a single process only handles one request at a time.

## Installation

To install Faucet, you first need to install [The Rust Programming Language](https://www.rust-lang.org/) on your system. In the future, platform specific binaries will be distributed.

Clone the repository and run `cargo install --path .` to install the `faucet` binary.

## Usage

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
faucet --dir /path/to/api
```

For more information on the available flags, run `faucet --help`.
