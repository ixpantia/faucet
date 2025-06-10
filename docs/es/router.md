# Modo Router de Faucet

El modo router de Faucet te permite servir múltiples aplicaciones distintas desde una única instancia de Faucet. Cada aplicación, o "ruta", puede tener su propia configuración (como tipo de aplicación, directorio de trabajo, número de workers y estrategia de balanceo de carga) y es accesible a través de un prefijo de ruta URL único.

Esto es particularly útil para:

 - Alojar múltiples aplicaciones Shiny, APIs de Plumber o documentos Quarto Shiny en el mismo servidor y puerto.
 - Desplegar diferentes versiones o configuraciones de la misma aplicación bajo diferentes rutas.
 - Consolidar tus despliegues de aplicaciones R en un único proceso de Faucet.

## Resumen en Video

Para una demostración visual de la característica del router de Faucet, revisa el siguiente video:

<iframe width="560" height="315" src="https://www.youtube.com/embed/hQEdbrb2iTc" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

## Activación del Modo Router

Para ejecutar Faucet en modo router, utilizas el subcomando `router`:

```bash
faucet router [OPCIONES]
```

La opción principal para el modo router es especificar el archivo de configuración:

*   **CLI:** `--conf <RUTA_AL_ARCHIVO_DE_CONFIGURACION>` o `-c <RUTA_AL_ARCHIVO_DE_CONFIGURACION>`
*   **Variable de Entorno:** `FAUCET_ROUTER_CONF=<RUTA_AL_ARCHIVO_DE_CONFIGURACION>`
*   **Por Defecto:** Si no se especifica, Faucet buscará un archivo llamado `frouter.toml` en el directorio de trabajo actual (`./frouter.toml`).

Las opciones globales de Faucet como `--host`, `--ip-from`, `--rscript`, `--quarto` y las configuraciones de telemetría (por ejemplo, `--pg-con-string`) se aplican a toda la instancia del router y a todas las rutas que gestiona.

## Archivo de Configuración (`frouter.toml`)

El modo router se configura mediante un archivo TOML. Este archivo debe definir un array llamado `route`, donde cada elemento del array es un objeto que configura una ruta de aplicación específica.

Aquí está la estructura de un único objeto de ruta dentro del archivo `frouter.toml`:

```toml
[[route]]
# El prefijo de la ruta URL para esta aplicación.
# Este prefijo DEBE terminar con una barra inclinada (ej., "/app/", "/api/v1/").
# Si es la ruta raíz, debe ser "/".
# (Requerido)
route = "/mi_aplicacion/"

# El tipo de aplicación.
# (Requerido)
# Posibles valores: "plumber", "shiny", "quarto-shiny"
# Alias como "Plumber", "Shiny", "QuartoShiny" también son aceptados.
server_type = "shiny"

# El directorio de trabajo para esta aplicación específica.
# Archivos como app.R o plumber.R se buscarán en relación con este directorio,
# o dentro de `app_dir` si se especifica. Las rutas pueden ser relativas (a donde está frouter.toml) o absolutas.
# (Opcional, por defecto es "." - el directorio donde se encuentra frouter.toml)
workdir = "./apps/mi_app_shiny"

# El subdirectorio dentro de `workdir` donde se encuentra el archivo principal de la aplicación (ej., app.R).
# Si tu app.R está directamente en `workdir`, puedes omitir esto o configurarlo como ".".
# (Opcional)
app_dir = "source" # Busca ./apps/mi_app_shiny/source/app.R

# El número de procesos worker a generar para esta aplicación.
# (Requerido)
workers = 2

# La estrategia de balanceo de carga para esta aplicación.
# (Opcional, los valores por defecto dependen del tipo de aplicación: "ip-hash" para shiny/quarto-shiny, "round-robin" para plumber)
# Posibles valores: "round-robin", "ip-hash", "cookie-hash"
strategy = "ip-hash"

# Ruta al documento Quarto (.qmd), requerido si server_type es "quarto-shiny".
# La ruta debe ser relativa a `workdir` o una ruta absoluta.
# (Opcional, pero requerido para quarto-shiny)
# qmd = "dashboard.qmd"
```

### Campos Explicados:

*   `route` (String, Requerido): El prefijo de la ruta URL. **Este prefijo debe terminar con una barra inclinada (ej., `/app/`, `/api/v1/`) a menos que sea la ruta raíz (`/`)**. Faucet dirigirá las solicitudes que comiencen con esta ruta a la aplicación configurada.
*   `server_type` (String, Requerido): Determina el tipo de aplicación R. Debe ser uno de `plumber`, `shiny` o `quarto-shiny`. Alias como `Plumber`, `Shiny`, `QuartoShiny` también son aceptados.
*   `workdir` (String, Opcional): El directorio de trabajo base para la aplicación. Si no se especifica, por defecto es el directorio donde se está ejecutando Faucet (típicamente donde se encuentra `frouter.toml`). Las rutas para `app_dir` y `qmd` se resuelven típicamente en relación con este.
*   `app_dir` (String, Opcional): Un subdirectorio dentro de `workdir` que contiene el archivo principal de la aplicación (por ejemplo, `app.R` para Shiny, `plumber.R` para Plumber). Por ejemplo, si `workdir = "./mi_coleccion_apps"` y `app_dir = "app_especifica_src"`, Faucet buscará `./mi_coleccion_apps/app_especifica_src/app.R`. Si el archivo principal está directamente en `workdir`, puedes omitir esto o usar `app_dir = "."`.
*   `workers` (Integer, Requerido): El número de procesos worker de R a lanzar para esta ruta específica. Debe ser un entero positivo.
*   `strategy` (String, Opcional): La estrategia de balanceo de carga para esta ruta.
    *   Para aplicaciones `shiny` y `quarto-shiny`, generalmente se recomienda `ip-hash` y es el valor por defecto para asegurar la persistencia de la sesión.
    *   Para APIs `plumber`, `round-robin` es el valor por defecto común.
    *   Opciones disponibles: `round-robin`, `ip-hash`, `cookie-hash`.
*   `qmd` (String, Opcional): Si `server_type` es `quarto-shiny`, este campo es requerido y debe especificar la ruta al archivo `.qmd`. Esta ruta es típicamente relativa a `workdir`.

**Importante:** Cada valor de `route` en el archivo de configuración debe ser único. Rutas duplicadas harán que Faucet termine con un error al iniciarse.

## Comportamiento del Enrutamiento y Eliminación de Prefijo de Ruta (Path Stripping)

Cuando Faucet recibe una solicitud HTTP en modo router:

1.  Itera a través de las definiciones `[[route]]` en `frouter.toml` **en el orden en que están definidas.**
2.  **Coincidencia de Rutas y Orden:**
    *   Para cada ruta definida, Faucet comprueba si la ruta URL de la solicitud entrante comienza con el prefijo `route` de la ruta.
    *   **Se utiliza la primera ruta que coincida.** Esto significa que el orden de tus rutas en `frouter.toml` es crítico. Las rutas más específicas (ej., `/app/feature1/`) deben listarse *antes* que las rutas más generales (ej., `/app/`) si comparten una ruta base común, para evitar que la ruta general "sombreé" a la específica. La ruta raíz `/` generalmente debe listarse al final.
3.  **Eliminación de Prefijo de Ruta (Path Stripping):**
    *   Todos los prefijos `route` (excepto la ruta raíz `/`) **deben terminar con una barra inclinada (`/`)**.
    *   Una vez que se encuentra una ruta coincidente, su prefijo `route` definido se elimina del inicio de la ruta URL de la solicitud.
    *   La parte restante de la ruta se reenvía entonces a la aplicación configurada para esa ruta.
    *   Ejemplo: Si se define `route = "/myapp/"`:
        *   Una solicitud a `/myapp/usuarios/1` hace que la aplicación vea `/usuarios/1`.
        *   Una solicitud a `/myapp/` (con la barra inclinada final) hace que la aplicación vea `/`.
    *   Ejemplo: Si se define `route = "/"`:
        *   Una solicitud a `/pagina` hace que la aplicación vea `/pagina`.
        *   Una solicitud a `/` hace que la aplicación vea `/`.
4.  Si se encuentra una ruta coincidente, la solicitud (con la ruta potencialmente modificada) se entrega a la instancia del servidor Faucet que gestiona esa aplicación específica, la cual luego aplica su estrategia de balanceo de carga configurada para seleccionar un worker.
5.  Si ninguna `route` configurada coincide con la ruta de la solicitud entrante, Faucet devuelve una respuesta `404 Not Found`.

## Ejemplo de `frouter.toml`

Este ejemplo se basa en `faucet-router-example` disponible en el repositorio de GitHub de Faucet bajo el directorio `examples/`. Para ejecutar este ejemplo, navega a `examples/faucet-router-example-main/` y ejecuta `faucet router`.

```toml
# frouter.toml
# Este archivo está ubicado en examples/faucet-router-example-main/

# Ruta para la aplicación Shiny "sliders".
# `workdir` está configurado como "./sliders", entonces Faucet busca app.R
# en examples/faucet-router-example-main/sliders/app.R
[[route]]
route = "/sliders/"
workers = 1
server_type = "shiny" # Nota: "Shiny" (con mayúscula) también es aceptado
workdir = "./sliders"

# Ruta para la aplicación Shiny "text".
# `workdir` por defecto es "." (donde está frouter.toml).
# `app_dir` es "./text", entonces Faucet busca app.R
# en examples/faucet-router-example-main/text/app.R
[[route]]
route = "/text/"
workers = 1
server_type = "shiny"
app_dir = "./text"

# Ruta para un documento Quarto Shiny.
# `workdir` es "./qmd".
# `qmd` especifica "old_faithful.qmd" relativo a workdir.
# Faucet busca examples/faucet-router-example-main/qmd/old_faithful.qmd
[[route]]
route = "/qmd/"
workers = 1
server_type = "quarto-shiny" # Nota: "QuartoShiny" (con mayúscula) también es aceptado
workdir = "./qmd"
qmd = "old_faithful.qmd"

# Ruta para una API de Plumber.
# `workdir` es "./api". Faucet busca plumber.R
# en examples/faucet-router-example-main/api/plumber.R
[[route]]
route = "/api/"
workers = 1
server_type = "plumber" # Nota: "Plumber" (con mayúscula) también es aceptado
workdir = "./api"
strategy = "round-robin"

# Ruta raíz para la aplicación Shiny principal.
# `workdir` por defecto es "." (donde está frouter.toml).
# Faucet busca app.R en examples/faucet-router-example-main/app.R
# Esta ruta se coloca al final para evitar solapar otras rutas específicas.
[[route]]
route = "/"
workers = 1
server_type = "shiny"
strategy = "cookie-hash"
```

Con la configuración anterior, si Faucet se está ejecutando en `http://localhost:3838` desde el directorio `examples/faucet-router-example-main/`:

 - Las solicitudes a `http://localhost:3838/sliders/` serían enrutadas a la aplicación Shiny en el subdirectorio `sliders`.
 - Las solicitudes a `http://localhost:3838/text/` serían enrutadas a la aplicación Shiny en el subdirectorio `text`.
 - Las solicitudes a `http://localhost:3838/qmd/` serían enrutadas al documento Quarto Shiny `old_faithful.qmd`.
 - Las solicitudes a `http://localhost:3838/api/echo?msg=hola` serían enrutadas a la API de Plumber en el subdirectorio `api` (la API vería `/echo?msg=hola`).
 - Las solicitudes a `http://localhost:3838/` serían enrutadas al `app.R` en la raíz del directorio `faucet-router-example-main`.

**Nota sobre el Orden de las Rutas:** Recuerda que si tienes rutas con rutas base superpuestas (ej., `/datos/especifico/` y `/datos/`), debes listar la ruta más específica (`/datos/especifico/`) *antes* que la ruta más general (`/datos/`) en tu archivo `frouter.toml`. De lo contrario, la ruta general `/datos/` coincidiría con las solicitudes destinadas a `/datos/especifico/`, y nunca se alcanzaría la ruta específica. La ruta raíz `/` típicamente debería ser la última entrada.

Este modo router proporciona una forma flexible de gestionar y servir múltiples aplicaciones R eficientemente usando una única instancia de Faucet.
