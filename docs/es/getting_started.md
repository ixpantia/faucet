## Inicio Rápido

Para usar faucet, asegúrate de que esté instalado. Si no lo está, consulta la [documentación oficial de instalación](./install.md).

### Modo Single Server

Una vez instalado, utiliza el siguiente comando para iniciar faucet con la configuración predeterminada:

```bash
# Iniciar faucet
faucet start
```

faucet se vinculará a `127.0.0.1:3838` y determinará automáticamente el número de hilos de trabajo según el número de CPU en la máquina host.

#### Ejecutar una Aplicación Shiny

Creemos una aplicación Shiny simple y despliégala usando faucet.

1. Crea una aplicación Shiny básica llamada `app.R`:

```R
# app.R
library(shiny)

ui <- fluidPage(
  shinyOutput("hello")
)

server <- function(input, output) {
  output$hello <- renderText({
    "¡Hola, faucet!"
  })
}

shinyApp(ui, server)
```

2. Guarda el código anterior en un archivo llamado `app.R`.

3. Inicia faucet en el mismo directorio que tu aplicación Shiny:

```bash
faucet start
```

faucet detectará automáticamente la aplicación Shiny y la desplegará.

4. Abre tu navegador web y dirígete a [http://127.0.0.1:3838](http://127.0.0.1:3838) para ver tu aplicación Shiny en acción.

## Ejecutar una Aplicación de Quarto

Para ejecutar una aplicación de Quarto usando faucet, sigue estos pasos:

1. Asegúrate de tener un archivo de documento Quarto, por ejemplo, `example.qmd`.

2. En el mismo directorio que tu documento Quarto, inicia faucet con la configuración de Quarto:

```bash
faucet start --qmd example.qmd --type quarto-shiny
```

Todos los demás argumentos aún persisten y se pueden personalizar según sea necesario.

faucet desplegará el documento Quarto como una aplicación Shiny.

3. Abre tu navegador web y navega a [http://127.0.0.1:3838](http://127.0.0.1:3838) para ver tu aplicación Quarto en acción.

#### Añadir más trabajadores

Si tu computadora tiene más de un núcleo de CPU, probablemente hayas visto que se crearon muchos trabajadores al iniciar faucet. Esto se debe a que faucet detecta automáticamente el número de núcleos de CPU en tu computadora y crea un trabajador por cada núcleo.

Para saber cuántos núcleos de CPU tienes, puedes ejecutar:

```bash
faucet start --help
```

En la salida, busca la opción -w, --workers <WORKERS>. El número predeterminado (default) se establece en la cantidad de núcleos de CPU detectados por Faucet.

Puedes personalizar el número de trabajadores utilizando la bandera `--workers`:

```bash
faucet start --workers 4
```

O configurando la variable de entorno `FAUCET_WORKERS`:

```bash
export FAUCET_WORKERS=4
faucet start
```

En ambos casos, faucet creará 4 trabajadores en puertos aleatorios disponibles. El tráfico se equilibrará entre todos los trabajadores según la dirección IP de la solicitud entrante. Esto significa que si tienes 4 trabajadores, podrás manejar 4 veces más solicitudes concurrentes que un solo trabajador.

### Modo Router

**¿Cuándo usar Router?**

- **Múltiples Aplicaciones:** Usa el modo Router cuando necesitas desplegar y gestionar múltiples aplicaciones en diferentes rutas, pero en un mismo puerto.

- **Gestión Centralizada:** Si deseas una configuración centralizada para dirigir las solicitudes a las aplicaciones correspondientes basadas en la ruta, Router es la opción adecuada.

- **Optimización de Recursos:** Router facilita la gestión y escalabilidad de varias aplicaciones al permitir una distribución eficiente de las solicitudes.

Para iniciar el modo Router de faucet, necesitamos primero un archivo de configuración en el cual estará la lógica del router `frouter.toml`. El archivo de configuración debe estar en la raíz del directorio de trabajo donde deseas ejecutar las aplicaciones. 

*Nota: Recuerda que faucet router detecta automáticamente el archivo app.R (Shiny), por este motivo, si existen muchas aplicaciones Shiny debemos especificarle la carpeta en donde se encuentra ese archivo app.R.*

Para explicar mejor la configuración tenemos un repositorio de ejemplo llamado [faucet-router-example](https://github.com/ixpantia/faucet-router-example). Este repositorio tiene diferentes aplicaciones (Quarto, Shiny y Plumper) por carpetas. 

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

Ejemplo `frouter.toml`:

```sh
# Por defecto, el `workdir` y `app_dir`
# es `.` (Aquí). Si no lo especificamos
# ejecuta la aplicación en el directorio actual.
[[route]]
route = "/"
workers = 1
server_type = "Shiny"


# En esta ruta usamos `workdir` para iniciar la sesión
# secundaria de R en un directorio de trabajo diferente.
[[route]]
route = "/sliders/"
workers = 1
server_type = "Shiny"
workdir = "./sliders"


# En esta ruta usamos `app_dir` para iniciar la sesión
# de R en el directorio de trabajo actual pero usar una aplicación en
# un directorio.
[[route]]
route = "/text/"
workers = 1
server_type = "Shiny"
app_dir = "./text"


# Demostración de cómo servir una API de Plumber
[[route]]
route = "/api/"
workers = 1
server_type = "Plumber"
workdir = "./api"


# Demostración de cómo servir una aplicación shiny de quarto
[[route]]
route = "/qmd/"
workers = 1
server_type = "QuartoShiny"
workdir = "./qmd"
qmd = "old_faithful.qmd"
```

El argumento `server_type` define que tipo de aplicación quieres desplegar, actualmente tenemos: `QuartoShiny`, `Shiny` y `Plumber`.

En este mismo archivo de configuración `frouter.toml`, podemos definir la cantidad de `workers` que necesita cada aplicación.

Ahora, para iniciar faucet en modo Router:

```sh
faucet router
```

#### Rutas:

Todas las aplicaciones estarán en el mismo puerto, pero con diferentes rutas, según el archivo de configuración. 

- Hello Shiny [`/`]: [`http://localhost:3838`](http://localhost:3838)
- Sliders Shiny [`/sliders/`]: [`http://localhost:3838/sliders/`](http://localhost:3838/sliders/)
- Text Shiny [`/text/`]: [`http://localhost:3838/text/`](http://localhost:3838/text/)
- Plumber API [`/api/`]: [`http://localhost:3838/api/__docs__/`](http://localhost:3838/api/__docs__/)
- Quarto Shiny App [`/qmd/`]: [`http://localhost:3838/qmd/`](http://localhost:3838/qmd/)



## Conclusión

¡Felicidades! Has comenzado a usar faucet y desplegado una o varias aplicaciones con muchos trabajadores.

¡Feliz programación con faucet!
