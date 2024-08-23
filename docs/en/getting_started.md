## Quick Start

To use faucet, ensure that it is installed. If not, refer to the [official installation documentation](./install.md).

Once installed, use the following command to start faucet with default settings:

```bash
# Start faucet
faucet start
```

faucet will bind to `127.0.0.1:3838` and automatically determine the number of worker threads based on the number of CPUs on the host machine.

## Running a Shiny Application

Let's create a simple Shiny application and deploy it using faucet.

1. Create a basic Shiny app named `app.R`:

```R
# app.R
library(shiny)

ui <- fluidPage(
  shinyOutput("hello")
)

server <- function(input, output) {
  output$hello <- renderText({
    "Hello, faucet!"
  })
}

shinyApp(ui, server)
```

2. Save the above code in a file named `app.R`.

3. Start faucet in the same directory as your Shiny app:

```bash
faucet start
```

faucet will automatically detect the Shiny app and deploy it.

4. Open your web browser and navigate to [http://127.0.0.1:3838](http://127.0.0.1:3838) to see your Shiny app in action.

## Running a Quarto Application

To run a Quarto application using faucet, follow these steps:

1. Ensure you have a Quarto document file, e.g., `example.qmd`.

2. In the same directory as your Quarto document, start faucet with the Quarto settings:

```bash
faucet start --qmd example.qmd --type quarto-shiny
```

All other arguments still persist and can be customized as needed.

faucet will deploy the Quarto document as a Shiny application.

3. Open your web browser and navigate to [http://127.0.0.1:3838](http://127.0.0.1:3838) to see your Quarto app in action.

### Adding more workers

If your computer has more than one CPU core, then you probably saw that
many workers were created when you started faucet. This is because faucet
automatically detects the number of CPU cores on your computer and creates
a worker for each core.

To know how many CPU cores you have, you can run the following commands:

- On Linux:
```bash
lscpu
```

- On Windows Powershell
```bash
Get-WmiObject -Class Win32_Processor | Select-Object NumberOfCores, NumberOfLogicalProcessors
```

You can customize the number of workers by using the `--workers` flag:

```bash
faucet start --workers 4
```

Or by setting the `FAUCET_WORKERS` environment variable:

```bash
export FAUCET_WORKERS=4
faucet start
```

In both cases, faucet will create 4 workers on random available ports.
Traffic will be load balanced across all workers according to the
IP address of the incoming request. This means that if you have 4 workers,
then you can handle 4 times as many concurrent requests as a single worker.

### Router Mode

**When to use Router?**

- **Multiple Applications:** Use Router mode when you need to deploy and manage multiple applications on different routes but on the same port.

- **Centralized Management:** If you desire a centralized configuration to route requests to the corresponding applications based on the route, Router is the appropriate option.

- **Resource Optimization:** Router facilitates the management and scalability of various applications by allowing an efficient distribution of requests.

To start faucet in Router mode, we first need a configuration file where the router logic `frouter.toml` will be placed. The configuration file must be in the root of the repository.

*Note: Remember that faucet router automatically detects the app.R (Shiny) file, so if there are many Shiny applications, we must specify the folder where that app.R file is located.*

To better explain the configuration, we have an example repository called [faucet-router-example](https://github.com/ixpantia/faucet-router-example). This repository has different applications (Quarto, Shiny, and Plumber) in separate folders.

```bash
│   .gitignore
│   faucet-router-example.Rproj
│   frouter.toml
│   README.md
│   
│   
│   app.R
│
├───sliders
│       app.R
│
└───text
│        app.R
├───api
│       plumber.R
│
├───qmd
│   │   old_faithful.qmd
│
```

Example `frouter.toml`:

```sh
# By default, the `workdir` and `app_dir`
# is `.` (Here). If not specified,
# runs the application in the current directory.
[[route]]
route = "/"
workers = 1
server_type = "Shiny"


# In this route, we use `workdir` to start the secondary
# R session in a different working directory.
[[route]]
route = "/sliders/"
workers = 1
server_type = "Shiny"
workdir = "./sliders"


# In this route, we use `app_dir` to start the R session
# in the current working directory but use an application in
# a directory.
[[route]]
route = "/text/"
workers = 1
server_type = "Shiny"
app_dir = "./text"


# Demonstration of how to serve a Plumber API
[[route]]
route = "/api/"
workers = 1
server_type = "Plumber"
workdir = "./api"


# Demonstration of how to serve a Quarto Shiny application
[[route]]
route = "/qmd/"
workers = 1
server_type = "QuartoShiny"
workdir = "./qmd"
qmd = "old_faithful.qmd"
```

The `server_type` argument defines the type of application you want to deploy; currently, we have: `QuartoShiny`, `Shiny`, and `Plumber`.

In the same configuration file `frouter.toml`, we can define the number of `workers` that each application needs.

Now, to start faucet in Router mode:

```sh
faucet router
```

#### Routes:

All the applications will be on the same port but with different routes, according to the configuration file.

- Hello Shiny [`/`]: [`http://localhost:3838`](http://localhost:3838)
- Sliders Shiny [`/sliders/`]: [`http://localhost:3838/sliders/`](http://localhost:3838/sliders/)
- Text Shiny [`/text/`]: [`http://localhost:3838/text/`](http://localhost:3838/text/)
- Plumber API [`/api/`]: [`http://localhost:3838/api/__docs__/`](http://localhost:3838/api/__docs__/)
- Quarto Shiny App [`/qmd/`]: [`http://localhost:3838/qmd/`](http://localhost:3838/qmd/)


## Conclusion

Congratulations! You have successfully started using faucet and deployed a
basic Shiny application with many workers.

Happy coding with faucet!
