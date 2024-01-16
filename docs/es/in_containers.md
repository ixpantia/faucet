# faucet en Contenedores (Docker)

Probablemente la manera más fácil y versátil de implementar faucet es construir una imagen de contenedor Linux y ejecutarla en un contenedor. Esto le permitirá ejecutar faucet en cualquier host de Linux que admita contenedores, como una computadora portátil, una máquina virtual o un servidor.

## Construir una Imagen de Docker para faucet

En esta sección, utilizarás la imagen disponible de faucet + R desde Docker Hub. Sin embargo, también puedes construir tu propia imagen si lo deseas. Debes seguir las instrucciones de instalación disponibles para tu sistema operativo para instalar Docker.

Para construir correctamente la imagen de faucet, debes tener en cuenta los siguientes pasos:

1. _Instalar Docker en tu máquina host._ Puedes encontrar las instrucciones específicas para tu sistema operativo en la [Guía de Instalación de Docker](https://docs.docker.com/engine/install/).
2. _Considerar las dependencias de tu aplicación en R._ Si estás utilizando paquetes R que requieren dependencias del sistema, deberás instalarlas en la imagen de Docker. También, ten en cuenta las versiones de R y las bibliotecas; te recomendamos utilizar [renv](https://rstudio.github.io/renv/articles/renv.html). Para este tutorial, asumiremos que ya estás utilizando `renv`.
3. _Ignorar archivos sensibles o innecesarios._ Puedes utilizar un archivo `.dockerignore` para ignorar archivos que no son necesarios en la imagen de Docker, o puedes especificar manualmente los archivos que deseas incluir en la imagen. En este caso, utilizaremos un archivo `.dockerignore` para ignorar dichos archivos.

### Una aplicación básica Shiny o Plumber API

En esta sección, arrancarás una aplicación Shiny básica o Plumber API para utilizar como ejemplo. Puedes usar tu propia aplicación o API, pero asegúrate de tener `renv` inicializado.

#### Aplicación Shiny

```r
# app.R
library(shiny)

ui <- fluidPage(
  titlePanel("¡Hola Shiny!"),
  sidebarLayout(
    sidebarPanel(
      sliderInput("obs", "Número de observaciones:", min = 10, max = 500, value = 100)
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

Después de guardar la aplicación, puedes ejecutarla localmente con:

```r
shiny::runApp()
```

Para asegurarte de que `renv` detecte todos los paquetes utilizados en la aplicación, debes crear un archivo `dependencies.R` con el siguiente contenido:

```r
# dependencies.R
library(shiny)
```

Ahora puedes inicializar `renv` e instalar los paquetes:

```r
renv::init()
renv::activate()
```

#### Plumber API

```r
# plumber.R
#* @get /echo
function(){
  list(msg = "¡Hola Mundo!")
}
```

Después de guardar la API, puedes ejecutarla localmente con:

```r
library(plumber)
# 'plumber.R' es la ubicación del archivo mostrado anteriormente
pr("plumber.R") %>%
  pr_run()
```

Para asegurarte de que `renv` detecte todos los paquetes utilizados en la API, debes crear un archivo `dependencies.R` con el siguiente contenido:

```r
# dependencies.R
library(plumber)
```

Ahora puedes inicializar `renv` e instalar los paquetes:

```r
renv::init()
renv::activate()
```

### Dockerfile

#### Dockerignore

El primer paso para construir nuestra imagen de Docker es crear un archivo `.dockerignore` en la raíz de nuestro proyecto. Este archivo contendrá los archivos que deseas ignorar al construir la imagen de Docker. En este caso, ignoraremos los siguientes archivos de `renv`:

```dockerignore
renv/library/
renv/local/
renv/cellar/
renv/lock/
renv/python/
renv/sandbox/
renv/staging/
```

Si este fuera un proyecto real, probablemente también ignorarías archivos como `.git`, `.Rproj.user`, `.DS_Store` y archivos sensibles como `.env`, `.htpasswd`, etc.

#### Escribir el Dockerfile

El primer paso para construir nuestra imagen de Docker es crear un archivo `Dockerfile` en la raíz de nuestro proyecto. Este archivo contendrá las instrucciones para construir nuestra imagen de Docker. En este caso, utilizarás la imagen [`ixpantia/faucet`](https://hub.docker.com/r/ixpantia/faucet) como base. Esta imagen se basa en la imagen [`rocker/r-ver`](https://hub.docker.com/r/rocker/r-ver), que es una imagen R mínima basada en Debian Linux.

```
FROM ixpantia/faucet:r4.3

# Algunas variables de entorno para indicar a `renv`
# instalar paquetes en la ubicación correcta
# y sin enlaces simbólicos innecesarios
ENV RENV_CONFIG_CACHE_SYMLINKS FALSE
ENV RENV_PATHS_LIBRARY /srv/faucet/renv/library

# Copias los archivos necesarios para arrancar `renv`
COPY ./renv.lock .
COPY ./renv ./renv
COPY ./.Rprofile .

# Instalas los paquetes
RUN Rscript -e "renv::restore()"

# Copias los archivos de la aplicación/API; en este caso,
# reemplaza `app.R` con `plumber.R` si estás utilizando
# una Plumber API
COPY ./app.R .

# Puedes ejecutar el contenedor como un usuario no root
# por razones de seguridad, aunque esto no es necesario.
# Puedes ignorar esto
RUN chown -R faucet:faucet /srv/faucet/
USER faucet
```

#### Construir la imagen de Docker

Ahora que tienes un `Dockerfile` y un archivo `.dockerignore`, puedes construir la imagen de Docker con el siguiente comando:

```bash
docker build -t my_faucet_app .
```

#### Ejecutar la imagen de Docker

Una vez construida la imagen, puedes ejecutarla con el siguiente comando:

```bash
docker run --rm -p 3838:3838 my_faucet_app
```

Ahora puedes acceder a tu aplicación/API en `http://localhost:3838`.

#### Controlar la instancia de faucet

Puedes controlar todos los aspectos de la instancia de faucet configurando
variables de entorno en el contenedor de Docker. Por ejemplo, si deseas cambiar
el número de trabajadores, puedes hacerlo configurando la variable de entorno
`FAUCET_WORKERS`:

```bash
docker run --rm -p 3838:3838 -e FAUCET_WORKERS=4 my_faucet_app
```

Si estás ejecutando la aplicación/API detrás de un proxy como Nginx, puedes
configurar la variable de entorno `FAUCET_IP_FROM` en `x-real-ip`
o `x-forwarded-for` para asegurarte de que faucet obtenga la dirección IP
correcta del cliente.

```bash
docker run --rm -p 3838:3838 -e FAUCET_IP_FROM=x-real-ip my_faucet_app
```
