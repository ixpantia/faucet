# faucet in Containers (Docker)

Probably the easiest and most versatile way to deploy faucet
is to build a Linux container image and run it in a container.
This will allow you to run faucet on any Linux host that supports
containers, like a laptop, a VM, or a server.

## Build a Docker Image for faucet

In this section, you will be using the available faucet + R image
from the Docker Hub. You can however, build your own image if you
want to. You should use the available installation instructions
for your OS to install Docker.

In order to properly build the faucet image, you will need to
take the following steps into account:

1. _Install Docker on your host machine._ You can find instructions
   on how to do this for your specific OS in the
   [Docker Installation Guide](https://docs.docker.com/engine/install/).
2. _Take your R application dependencies into account._ If you are
   using R packages that require system dependencies, you will
   need to install them in the Docker image. Also, take the R
   and library versions into account, we highly recommend you
   use [renv](https://rstudio.github.io/renv/articles/renv.html).
   For this tutorial we will assume you are using `renv` already.
3. _Ignore sensitive or unnecessary files._ You can use a `.dockerignore`
   file to ignore files that are not needed in the Docker image or
   you can manually specify the files you want to include in the
   image. In this case, we will use a `.dockerignore` file to ignore
   said files.

### A basic Shiny app or Plumber API

In this section, you will bootstrap a basic Shiny app or Plumber API
to use as an example. You can use your own app or API, but make sure
you have `renv` initialized.

#### Shiny app

```r
# app.R
library(shiny)

ui <- fluidPage(
  titlePanel("Hello Shiny!"),
  sidebarLayout(
    sidebarPanel(
      sliderInput("obs", "Number of observations:", min = 10, max = 500, value = 100)
    ),
    mainPanel(
      plotOutput("distPlot")
    )
  )
)

server <- function(input, output) {
  output$distPlot <- renderPlot({
    hist(rnorm(input$obs))
  })
}

shinyApp(ui = ui, server = server)
```

After saving the app, you can run it locally with:

```r
shiny::runApp()
```

To make absolutely sure that `renv` detects all the packages used
in the app, you should create a `dependencies.R` file with the
following contents:

```r
# dependencies.R
library(shiny)
```

Now, you can initialize `renv` and install the packages:

```r
renv::init()
renv::activate()
```

#### Plumber API

```r
# plumber.R
#* @get /echo
function(){
  list(msg = "Hello World!")
}
```

After saving the API, you can run it locally with:

```r
library(plumber)
# 'plumber.R' is the location of the file shown above
pr("plumber.R") %>%
  pr_run()
```

To make absolutely sure that `renv` detects all the packages used
in the API, you should create a `dependencies.R` file with the
following contents:

```r
# dependencies.R
library(plumber)
```

Now, you can initialize `renv` and install the packages:

```r
renv::init()
renv::activate()
```

### Dockerfile

#### Dockerignore

The first step to building our Docker image is to create a `.dockerignore`
file in the root of our project. This file will contain the files
that you want to ignore when building the Docker image. In this case,
we will ignore the following `renv` files:

```dockerignore
renv/library/
renv/local/
renv/cellar/
renv/lock/
renv/python/
renv/sandbox/
renv/staging/
```

If this were a real project, you would probably also ignore files
like `.git`, `.Rproj.user`, `.DS_Store`, and sensitive files like
`.env`, `.htpasswd`, etc.


#### Writing the Dockerfile

The first step to building our Docker image is to create a `Dockerfile`
in the root of our project. This file will contain the instructions
to build our Docker image. In this case, you will use the
[`andyquinterom/faucet`](https://hub.docker.com/r/andyquinterom/faucet)
image as our base image. This image is based on the
[`rocker/r-ver`](https://hub.docker.com/r/rocker/r-ver) image,
which is a minimal R image based on Debian Linux.

```dockerfile
FROM andyquinterom/faucet:0.3.1-r4.3

# Some environment variables to tell `renv`
# to install packages in the correct location
# and without unnecessary symlinks
ENV RENV_CONFIG_CACHE_SYMLINKS FALSE
ENV RENV_PATHS_LIBRARY /srv/faucet/renv/library

# You copy the necessary files to bootstrap `renv`
COPY ./renv.lock .
COPY ./renv ./renv
COPY ./.Rprofile .

# You install the packages
RUN Rscript -e "renv::restore()"

# Copy the app/API files in this case replace
# `app.R` with `plumber.R` if you are using a
# Plumber API
COPY ./app.R .

# You can run the container as a non-root user
# for security reasons if we want to though
# this is not necessary. You could ignore this
RUN chown -R faucet:faucet /srv/faucet/
USER faucet
```

#### Building the Docker image

Now that you have a `Dockerfile` and a `.dockerignore` file, you
can build the Docker image with the following command:

```bash
docker build -t my_faucet_app .
```

#### Running the Docker image

Once the image is built, you can run it with the following command:

```bash
docker run --rm -p 3838:3838 my_faucet_app
```

You can now access your app/API at `http://localhost:3838`.

#### Controlling the faucet instance

You can control every aspect of the faucet instance by setting
environment variables in the Docker container. For example, if
you want to change the number of workers, you can do so by setting
the `FAUCET_WORKERS` environment variable:

```bash
docker run --rm -p 3838:3838 -e FAUCET_WORKERS=4 my_faucet_app
```

If you are running the app/API behind a proxy like Nginx, you
can set the `FAUCET_IP_FROM` environment variable to `x-real-ip`
or `x-forwarded-for` to make sure faucet gets the correct IP
address of the client.

```bash
docker run --rm -p 3838:3838 -e FAUCET_IP_FROM=x-real-ip my_faucet_app
```
