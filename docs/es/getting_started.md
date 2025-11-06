## Inicio Rápido

Para usar faucet, asegúrate de que esté instalado. Si no es así, consulta la [documentación oficial de instalación](./install.md).

Una vez instalado, utiliza el siguiente comando para iniciar faucet con la configuración predeterminada:

```bash
# Iniciar faucet
faucet start
```

faucet se vinculará a `127.0.0.1:3838` y determinará automáticamente el número de hilos de trabajo basándose en el número de CPUs en la máquina anfitriona.

## Ejecutando una Aplicación Shiny

Vamos a crear una aplicación Shiny simple y a desplegarla usando faucet.

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

4. Abre tu navegador web y navega a [http://127.0.0.1:3838](http://127.0.0.1:3838) para ver tu aplicación Shiny en acción.

## Ejecutando una Aplicación Quarto

Para ejecutar una aplicación Quarto usando faucet, sigue estos pasos:

1. Asegúrate de tener un archivo de documento Quarto, por ejemplo, `example.qmd`.

2. En el mismo directorio que tu documento Quarto, inicia faucet con la configuración de Quarto:

```bash
faucet start --qmd example.qmd --type quarto-shiny
```

Todos los demás argumentos persisten y pueden ser personalizados según sea necesario.

faucet desplegará el documento Quarto como una aplicación Shiny.

3. Abre tu navegador web y navega a [http://127.0.0.1:3838](http://127.0.0.1:3838) para ver tu aplicación Quarto en acción.

## Ejecutando una Aplicación FastAPI

faucet también puede servir aplicaciones de Python construidas con FastAPI. Esta característica depende de que `uv` esté disponible en el `PATH` de tu sistema.

1.  Crea una aplicación FastAPI simple. Crea un archivo llamado `main.py` en el directorio de tu proyecto:

    ```python
    # main.py
    from fastapi import FastAPI

    app = FastAPI()

    @app.get("/")
    def read_root():
        return {"Hola": "Faucet"}
    ```

2.  Necesitarás `uvicorn` y `fastapi` en tu entorno de Python. Si estás usando `uv`, puedes instalarlos con:

    ```bash
    uv pip install fastapi uvicorn
    ```

3.  Inicia faucet y apúntalo a tu aplicación FastAPI:

    ```bash
    faucet start --type fast-api --dir .
    ```
    faucet buscará un archivo `main.py` en el directorio especificado y luego lo servirá usando `uv run uvicorn main:app`.

4.  Abre tu navegador web y navega a [http://127.0.0.1:3838](http://127.0.0.1:3838) para ver la respuesta de tu aplicación FastAPI.

## Ejecutando un Script de R

Puedes ejecutar scripts de R arbitrarios usando el subcomando `rscript`. Faucet gestionará la ejecución del script.

1.  Crea un script de R, por ejemplo `task.R`:

    ```R
    # task.R
    print("Ejecutando una tarea en R.")
    Sys.sleep(5)
    print("Tarea finalizada.")
    ```

2.  Ejecuta el script usando faucet:

    ```bash
    faucet rscript task.R
    ```
    Esto ejecutará `Rscript task.R` bajo la gestión de faucet. Puedes pasar cualquier argumento a tu script como lo harías normalmente.

## Ejecutando un Script de Python

De manera similar, puedes ejecutar scripts de Python o cualquier comando `uv` usando el subcomando `uv`. Esto requiere que `uv` esté instalado y disponible en tu `PATH`.

1.  Crea un script de Python, por ejemplo `task.py`:

    ```python
    # task.py
    import time
    print("Ejecutando una tarea en Python.")
    time.sleep(5)
    print("Tarea finalizada.")
    ```

2.  Ejecuta el script usando faucet:

    ```bash
    faucet uv run task.py
    ```
    Esto ejecutará `uv run task.py`. Se pueden pasar cualquier argumento a `uv`. Por ejemplo, para instalar un paquete en el entorno actual:

    ```bash
    faucet uv pip install requests
    ```

### Añadiendo más workers

Si tu computadora tiene más de un núcleo de CPU, probablemente viste que se crearon muchos workers cuando iniciaste faucet. Esto se debe a que faucet detecta automáticamente el número de núcleos de CPU en tu computadora y crea un worker por cada núcleo.

Para saber cuántos núcleos de CPU tienes, puedes ejecutar:

```bash
faucet start -- help
```

En la salida, busca la bandera -w, --workers <WORKERS>. El número predeterminado se establece en el número de núcleos de CPU detectados por Faucet.

Puedes personalizar el número de workers usando la bandera `--workers`:

```bash
faucet start --workers 4
```

O estableciendo la variable de entorno `FAUCET_WORKERS`:

```bash
export FAUCET_WORKERS=4
faucet start
```

En ambos casos, faucet creará 4 workers en puertos aleatorios disponibles. El tráfico será balanceado a través de todos los workers según la dirección IP de la solicitud entrante. Esto significa que si tienes 4 workers, puedes manejar 4 veces más solicitudes concurrentes que un solo worker.

### Modo Router

**¿Cuándo usar el Router?**

- **Múltiples Aplicaciones:** Usa el modo Router cuando necesites desplegar y gestionar múltiples aplicaciones en diferentes rutas pero en el mismo puerto.

- **Gestión Centralizada:** Si deseas una configuración centralizada para enrutar las solicitudes a las aplicaciones correspondientes según la ruta, el Router es la opción apropiada.

- **Optimización de Recursos:** El Router facilita la gestión y escalabilidad de varias aplicaciones al permitir una distribución eficiente de las solicitudes.

Para iniciar faucet en modo Router, primero necesitamos un archivo de configuración donde se colocará la lógica del router `frouter.toml`. El archivo de configuración debe estar en la raíz del directorio de trabajo donde deseas ejecutar las aplicaciones.

*Nota: Recuerda que el router de faucet detecta automáticamente el archivo app.R (Shiny), por lo que si hay muchas aplicaciones Shiny, debemos especificar la carpeta donde se encuentra ese archivo app.R.*

Para explicar mejor la configuración, tenemos un repositorio de ejemplo llamado [faucet-router-example](https://github.com/ixpantia/faucet-router-example). Este repositorio tiene diferentes aplicaciones (Quarto, Shiny y Plumber) en carpetas separadas.

```bash
│   .gitignore
│   faucet-router-example.Rproj
│   frouter.toml
│   README.md
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
├───py-api
│   │   main.py
```

Ejemplo de `frouter.toml`:

```toml
# Por defecto, `workdir` y `app_dir`
# es `.` (Aquí). Si no se especifica,
# ejecuta la aplicación en el directorio actual.
[[route]]
route = "/"
workers = 1
server_type = "Shiny"


# En esta ruta, usamos `workdir` para iniciar la sesión
# secundaria de R en un directorio de trabajo diferente.
[[route]]
route = "/sliders/"
workers = 1
server_type = "Shiny"
workdir = "./sliders"


# En esta ruta, usamos `app_dir` para iniciar la sesión de R
# en el directorio de trabajo actual pero usando una aplicación en
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


# Demostración de cómo servir una aplicación Quarto Shiny
[[route]]
route = "/qmd/"
workers = 1
server_type = "QuartoShiny"
workdir = "./qmd"
qmd = "old_faithful.qmd"

# Demostración de cómo servir una aplicación FastAPI
[[route]]
route = "/py-api/"
workers = 1
server_type = "FastAPI"
workdir = "./py-api"
```

El argumento `server_type` define el tipo de aplicación que deseas desplegar; actualmente, tenemos: `QuartoShiny`, `Shiny`, `Plumber`, y `FastAPI`.

En el mismo archivo de configuración `frouter.toml`, podemos definir el número de `workers` que necesita cada aplicación.

Ahora, para iniciar faucet en modo Router:

```sh
faucet router
```

#### Rutas:

Todas las aplicaciones estarán en el mismo puerto pero con diferentes rutas, según el archivo de configuración.

- Hello Shiny [`/`]: [`http://localhost:3838`](http://localhost:3838)
- Sliders Shiny [`/sliders/`]: [`http://localhost:3838/sliders/`](http://localhost:3838/sliders/)
- Text Shiny [`/text/`]: [`http://localhost:3838/text/`](http://localhost:3838/text/)
- Plumber API [`/api/`]: [`http://localhost:3838/api/__docs__/`](http://localhost:3838/api/__docs__/)
- Quarto Shiny App [`/qmd/`]: [`http://localhost:3838/qmd/`](http://localhost:3838/qmd/)
- FastAPI App [`/py-api/`]: [`http://localhost:3838/py-api/`](http://localhost:3838/py-api/)


## Conclusión

¡Felicidades! Has comenzado a usar faucet con éxito y has desplegado una aplicación Shiny básica con muchos workers.

¡Feliz codificación con faucet!