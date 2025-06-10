# Faucet Router Mode

Faucet's router mode allows you to serve multiple distinct applications from a single Faucet instance. Each application, or "route," can have its own configuration (like application type, working directory, number of workers, and load balancing strategy) and is accessible via a unique URL path prefix.

This is particularly useful for:

 - Hosting multiple Shiny apps, Plumber APIs, or Quarto Shiny documents on the same server and port.
 - Deploying different versions or configurations of the same application under different paths.
 - Consolidating your R application deployments into a single Faucet process.

## Video Overview

For a visual demonstration of the Faucet router feature, check out the following video:

<iframe width="560" height="315" src="https://www.youtube.com/embed/hQEdbrb2iTc" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

## Activating Router Mode

To run Faucet in router mode, you use the `router` subcommand:

```bash
faucet router [OPTIONS]
```

The primary option for router mode is to specify the configuration file:

*   **CLI:** `--conf <PATH_TO_CONFIG_FILE>` or `-c <PATH_TO_CONFIG_FILE>`
*   **Environment Variable:** `FAUCET_ROUTER_CONF=<PATH_TO_CONFIG_FILE>`
*   **Default:** If not specified, Faucet will look for a file named `frouter.toml` in the current working directory (`./frouter.toml`).

Global Faucet options such as `--host`, `--ip-from`, `--rscript`, `--quarto`, and telemetry settings (e.g., `--pg-con-string`) apply to the entire router instance and all routes it manages.

## Configuration File (`frouter.toml`)

The router mode is configured using a TOML file. This file must define an array named `route`, where each element in the array is an object configuring a specific application route.

Here's the structure of a single route object within the `frouter.toml` file:

```toml
[[route]]
# The URL path prefix for this application.
# This prefix MUST end with a trailing slash (e.g., "/app/", "/api/v1/").
# If it's the root path, it should be "/".
# (Required)
route = "/my_application/"

# The type of application.
# (Required)
# Possible values: "plumber", "shiny", "quarto-shiny"
# Aliases like "Plumber", "Shiny", "QuartoShiny" are also accepted.
server_type = "shiny"

# The working directory for this specific application.
# Files like app.R or plumber.R will be looked for relative to this directory,
# or within the `app_dir` if specified. Paths can be relative (to where frouter.toml is) or absolute.
# (Optional, defaults to "." - the directory where frouter.toml is located)
workdir = "./apps/my_shiny_app"

# The subdirectory within `workdir` where the application's main file (e.g., app.R) is located.
# If your app.R is directly in `workdir`, you can omit this or set it to ".".
# (Optional)
app_dir = "source" # Looks for ./apps/my_shiny_app/source/app.R

# The number of worker processes to spawn for this application.
# (Required)
workers = 2

# The load balancing strategy for this application.
# (Optional, defaults depend on application type: "ip-hash" for shiny/quarto-shiny, "round-robin" for plumber)
# Possible values: "round-robin", "ip-hash", "cookie-hash"
strategy = "ip-hash"

# Path to the Quarto document (.qmd file), required if server_type is "quarto-shiny".
# The path should be relative to `workdir` or an absolute path.
# (Optional, but required for quarto-shiny)
# qmd = "dashboard.qmd"
```

### Fields Explained:

*   `route` (String, Required): The URL path prefix. **This prefix must end with a trailing slash (e.g., `/app/`, `/api/v1/`) unless it is the root route (`/`)**. Faucet will direct requests starting with this path to the configured application.
*   `server_type` (String, Required): Determines the type of R application. Must be one of `plumber`, `shiny`, or `quarto-shiny`. Aliases like `Plumber`, `Shiny`, `QuartoShiny` are also accepted.
*   `workdir` (String, Optional): The base working directory for the application. If not specified, it defaults to the directory where Faucet is running (typically where `frouter.toml` is located). Paths for `app_dir` and `qmd` are typically resolved relative to this.
*   `app_dir` (String, Optional): A subdirectory within `workdir` that contains the application's main file (e.g., `app.R` for Shiny, `plumber.R` for Plumber). For example, if `workdir = "./my_app_collection"` and `app_dir = "specific_app_src"`, Faucet will look for `./my_app_collection/specific_app_src/app.R`. If the main file is directly in `workdir`, you can omit this or use `app_dir = "."`.
*   `workers` (Integer, Required): The number of R worker processes to launch for this specific route. Must be a positive integer.
*   `strategy` (String, Optional): The load balancing strategy for this route.
    *   For `shiny` and `quarto-shiny` apps, `ip-hash` is generally recommended and is the default to ensure session persistence.
    *   For `plumber` APIs, `round-robin` is the default.
    *   Available options: `round-robin`, `ip-hash`, `cookie-hash`.
*   `qmd` (String, Optional): If `server_type` is `quarto-shiny`, this field is required and must specify the path to the `.qmd` file. This path is typically relative to `workdir`.

**Important:** Each `route` value in the configuration file must be unique. Duplicate routes will cause Faucet to exit with an error on startup.

## Routing Behavior and Path Stripping

When Faucet receives an HTTP request in router mode:

1.  It iterates through the `[[route]]` definitions in `frouter.toml` **in the order they are defined.**
2.  **Route Matching and Order:**
    *   For each defined route, Faucet checks if the incoming request's URL path starts with the route's `route` prefix.
    *   **The first route that matches is used.** This means the order of your routes in `frouter.toml` is critical. More specific routes (e.g., `/app/feature1/`) must be listed *before* more general routes (e.g., `/app/`) if they share a common base path, to prevent the general route from "shadowing" the specific one. The root route `/` should generally be listed last.
3.  **Path Stripping:**
    *   All `route` prefixes (except for the root route `/`) **must end with a trailing slash (`/`)**.
    *   Once a matching route is found, its defined `route` prefix is stripped from the beginning of the request's URL path.
    *   The remaining part of the path is then forwarded to the application configured for that route.
    *   Example: If `route = "/myapp/"` is defined:
        *   A request to `/myapp/users/1` results in the application seeing `/users/1`.
        *   A request to `/myapp/` (with the trailing slash) results in the application seeing `/`.
    *   Example: If `route = "/"` is defined:
        *   A request to `/page` results in the application seeing `/page`.
        *   A request to `/` results in the application seeing `/`.
4.  If a matching route is found, the request (with the potentially modified path) is handed over to the Faucet server instance managing that specific application, which then applies its configured load balancing strategy to select a worker.
5.  If no configured `route` matches the incoming request's path, Faucet returns a `404 Not Found` response.

## Example `frouter.toml`

This example is based on the `faucet-router-example` available in the Faucet GitHub repository under the `examples/` directory. To run this example, navigate to `examples/faucet-router-example-main/` and run `faucet router`.

```toml
# frouter.toml
# This file is located in examples/faucet-router-example-main/

# Route for the "sliders" Shiny application.
# `workdir` is set to "./sliders", so Faucet looks for app.R
# in examples/faucet-router-example-main/sliders/app.R
[[route]]
route = "/sliders/"
workers = 1
server_type = "shiny" # Note: "Shiny" (capitalized) is also accepted
workdir = "./sliders"

# Route for the "text" Shiny application.
# `workdir` defaults to "." (where frouter.toml is).
# `app_dir` is "./text", so Faucet looks for app.R
# in examples/faucet-router-example-main/text/app.R
[[route]]
route = "/text/"
workers = 1
server_type = "shiny"
app_dir = "./text"

# Route for a Quarto Shiny document.
# `workdir` is "./qmd".
# `qmd` specifies "old_faithful.qmd" relative to workdir.
# Faucet looks for examples/faucet-router-example-main/qmd/old_faithful.qmd
[[route]]
route = "/qmd/"
workers = 1
server_type = "quarto-shiny" # Note: "QuartoShiny" (capitalized) is also accepted
workdir = "./qmd"
qmd = "old_faithful.qmd"

# Route for a Plumber API.
# `workdir` is "./api". Faucet looks for plumber.R
# in examples/faucet-router-example-main/api/plumber.R
[[route]]
route = "/api/"
workers = 1
server_type = "plumber" # Note: "Plumber" (capitalized) is also accepted
workdir = "./api"
strategy = "round-robin"

# Root route for the main Shiny application.
# `workdir` defaults to "." (where frouter.toml is).
# Faucet looks for app.R in examples/faucet-router-example-main/app.R
# This route is placed last to avoid shadowing other specific routes.
[[route]]
route = "/"
workers = 1
server_type = "shiny"
strategy = "cookie-hash"
```

With the configuration above, if Faucet is running on `http://localhost:3838` from the `examples/faucet-router-example-main/` directory:

 - Requests to `http://localhost:3838/sliders/` would be routed to the Shiny app in the `sliders` subdirectory.
 - Requests to `http://localhost:3838/text/` would be routed to the Shiny app in the `text` subdirectory.
 - Requests to `http://localhost:3838/qmd/` would be routed to the `old_faithful.qmd` Quarto Shiny document.
 - Requests to `http://localhost:3838/api/echo?msg=hello` would be routed to the Plumber API in the `api` subdirectory (the API would see `/echo?msg=hello`).
 - Requests to `http://localhost:3838/` would be routed to the `app.R` in the root of the `faucet-router-example-main` directory.

**Note on Route Order:** Remember that if you have routes with overlapping base paths (e.g., `/data/specific/` and `/data/`), you must list the more specific route (`/data/specific/`) *before* the more general route (`/data/`) in your `frouter.toml` file. Otherwise, the general route `/data/` would match requests intended for `/data/specific/`, and the specific route would never be reached. The root route `/` should typically be the last entry.

This router mode provides a flexible way to manage and serve multiple R applications efficiently using a single Faucet instance.

