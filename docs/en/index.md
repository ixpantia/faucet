# faucet ![logo](figures/faucet.png){ align=right height=139 width=120 }

<!-- badges: start -->
[![Crates.io](https://img.shields.io/crates/v/faucet-server.svg)](https://crates.io/crates/faucet-server)
<!-- badges: end -->

Fast, Asynchronous, Concurrent R Application Deployment

---

## Overview

Welcome to faucet, your high-performance solution for deploying Plumber APIs and Shiny Applications with speed and efficiency. faucet is a Rust-based server that offers Round Robin, IP Hash and Cookie Hash load balancing, ensuring seamless scaling and distribution of your R applications. Whether you're a data scientist, developer, or DevOps enthusiast, faucet simplifies deployment, making it easy to manage replicas and balance loads effectively.

## Features

- **High Performance:** faucet leverages Rust's speed for smooth and efficient execution of R applications.
- **Load Balancing:** Choose Round Robin, IP Hash or Cookie Hash load balancing for optimal resource utilization.
- **Replicas:** Scale Plumber APIs and Shiny Applications effortlessly with multiple replicas.
- **Simplified Deployment:** faucet streamlines the deployment process for quick setup.
- **Asynchronous & Concurrent:** Utilizes asynchronous and concurrent processing for resource efficiency and responsive request handling.
- **Structured Event Tracing:** Gain deep insights into your Shiny applications with detailed, machine-readable logs stored directly in your database.


## Installation

For installation options, refer to [Installation](./install.md).

## Usage

For detailed usage instructions, refer to [Getting Started](./getting_started.md).

## With Docker

faucet is also available as a Docker image, for detailed usage instructions with
Docker, refer to [faucet in Containers](./in_containers.md).
