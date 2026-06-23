# Opciones / Configuración

Esta sección cubre todas las opciones configurables por el usuario para faucet.

## Opciones Globales

Estas opciones se pueden usar con los subcomandos `start` y `router`.

### Host

- CLI: `--host`
- Entorno: `FAUCET_HOST`
- Predeterminado: `127.0.0.1:3838`

El host y el puerto al que se vinculará el servidor faucet. Si se ejecuta en un contenedor, esto debería establecerse en `0.0.0.0:3838` para permitir el acceso externo.

### IP From (Cómo determinar la IP del cliente)

- CLI: `--ip-from` o `-i`
- Entorno: `FAUCET_IP_FROM`
- Predeterminado: `client`
- Valores posibles:
  - `client`
  - `x-forwarded-for`
  - `x-real-ip`

Cómo determinar la IP del cliente. Esto se utiliza para determinar la IP para la estrategia de IP Hash y para el registro de solicitudes HTTP. Si estás ejecutando faucet directamente para los usuarios finales, deberías usar `client`. Si estás ejecutando faucet detrás de un proxy inverso como _nginx_, deberías usar `x-forwarded-for` o `x-real-ip`.

> **Nota:** Si estás ejecutando faucet detrás de un proxy inverso, asegúrate de establecer correctamente la cabecera `X-Forwarded-For` o `X-Real-IP` en la configuración de tu proxy inverso.

### Rscript (Definir un binario/ejecutable `Rscript` personalizado)

- CLI: `--rscript` o `-r`
- Entorno: `FAUCET_RSCRIPT`
- Predeterminado: `Rscript`

El binario/ejecutable `Rscript` a utilizar. Esto es útil si necesitas tener varias versiones de R instaladas en tu sistema. Debe ser la ruta completa al binario/ejecutable `Rscript` o un alias que esté disponible en tu `$PATH`. Esto también es útil en plataformas como _Windows_ donde el binario/ejecutable `Rscript` puede no estar disponible en el `$PATH`.

### Quarto (Definir un binario/ejecutable `quarto` personalizado)

- CLI: `--quarto` o `-q`
- Entorno: `FAUCET_QUARTO`
- Predeterminado: `quarto`

El binario/ejecutable `quarto` a utilizar. Esto es útil si tienes varias versiones de Quarto instaladas o si no está en tu `$PATH`.

### Uv (Definir un binario/ejecutable `uv` personalizado)

- CLI: `--uv`
- Entorno: `FAUCET_UV`
- Predeterminado: `uv`

El binario/ejecutable `uv` a utilizar. Esto es útil si tienes múltiples versiones de `uv` instaladas, o si no está en el `PATH` de tu sistema. `uv` es requerido para ejecutar aplicaciones FastAPI y subcomandos `uv`.

### Log File (Redirigir el registro a un archivo)

- CLI: `--log-file` o `-l`
- Entorno: `FAUCET_LOG_FILE`
- Predeterminado: `None`

Si estableces esta variable, se desactivarán los colores en `stderr` y se guardará toda la salida en la ruta especificada. Esto añadirá contenido, no sobrescribirá archivos existentes.

### Max Log File Size

- CLI: `--max-log-file-size` o `-m`
- Entorno: `FAUCET_MAX_LOG_FILE_SIZE`
- Predeterminado: `None`

El tamaño máximo del archivo de registro antes de la rotación (p. ej., 10M, 1GB). Requiere que `log-file` esté configurado.

### Logging Level

- Entorno: `FAUCET_LOG`
- Predeterminado: `INFO`
- Valores posibles:
  - `ERROR`
  - `WARN`
  - `INFO`
  - `DEBUG`
  - `TRACE`

El nivel de registro a utilizar. Esta variable de entorno establece la verbosidad global del registro. Consulta la sección de [registro](./logging.md) para más información.
**Nota:** Aunque esta variable de entorno es funcional, las aplicaciones más nuevas podrían preferir un control más granular a través de archivos de configuración de logger dedicados o configuraciones específicas de la biblioteca si están disponibles. Las opciones de CLI `--log-file` y `--max-log-file-size` proporcionan control directo sobre el registro basado en archivos.

### Shutdown

- CLI: `--shutdown`
- Entorno: `FAUCET_SHUTDOWN`
- Predeterminado: `immediate`
- Valores posibles:
  - `immediate`
  - `graceful`

La estrategia utilizada para apagar faucet. `immediate` termina cada conexión activa y apaga el proceso. `graceful` espera a que todas las conexiones se cierren antes de apagarse.

### Max Message Size

- CLI: `--max-message-size`
- Entorno: `FAUCET_MAX_MESSAGE_SIZE`
- Predeterminado: `None`

Tamaño máximo de un mensaje de WebSocket. Esto es útil para la prevención de ataques DDOS. Si no se establece, no hay límite de tamaño.

### Telemetría: Cadena de Conexión de PostgreSQL

- CLI: `--pg-con-string`
- Entorno: `FAUCET_TELEMETRY_POSTGRES_STRING`
- Predeterminado: `None`

Cadena de conexión a una base de datos PostgreSQL para guardar eventos HTTP. Si se proporciona, faucet intentará registrar los eventos HTTP en esta base de datos.

### Telemetría: Namespace

- CLI: `--telemetry-namespace`
- Entorno: `FAUCET_TELEMETRY_NAMESPACE`
- Predeterminado: `faucet`

Espacio de nombres bajo el cual se guardan los eventos HTTP en PostgreSQL.

### Telemetría: Versión

- CLI: `--telemetry-version`
- Entorno: `FAUCET_TELEMETRY_VERSION`
- Predeterminado: `None`

Representa la versión del código fuente del servicio que se está ejecutando. Esto es útil para filtrar datos de telemetría.

### Telemetría: Certificado SSL de PostgreSQL

- CLI: `--pg-sslcert`
- Entorno: `FAUCET_TELEMETRY_POSTGRES_SSLCERT`
- Predeterminado: `None`

Ruta a un archivo de certificado de CA para verificar el servidor PostgreSQL al usar SSL/TLS. Requerido si `--pg-sslmode` se establece en `verify-ca` o `verify-full`. El certificado debe estar en formato PEM o DER.

### Telemetría: Modo SSL de PostgreSQL

- CLI: `--pg-sslmode`
- Entorno: `FAUCET_TELEMETRY_POSTGRES_SSLMODE`
- Predeterminado: `prefer`
- Valores posibles:
  - `disable`
  - `prefer`
  - `require`
  - `verify-ca`
  - `verify-full`

Controla el comportamiento de SSL/TLS para la conexión de PostgreSQL. Si se establece en `verify-ca` o `verify-full`, se debe proporcionar un certificado de CA a través de `--pg-sslcert` o `FAUCET_TELEMETRY_POSTGRES_SSLCERT`.

## Opciones del Subcomando `start`

Estas opciones son específicas del subcomando `start`, utilizado para ejecutar un servidor faucet estándar.

### Workers

- CLI: `--workers` o `-w`
- Entorno: `FAUCET_WORKERS`
- Predeterminado: El número de CPUs disponibles para el proceso

El número de procesos de trabajo a generar. En una carga de trabajo limitada por la CPU, esto debería establecerse en el número de CPUs disponibles para el proceso. En una carga de trabajo limitada por E/S, esto podría establecerse en un número mayor.

### Strategy

- CLI: `--strategy` o `-s`
- Entorno: `FAUCET_STRATEGY`
- Predeterminado: `round-robin`
- Valores posibles:
  - `round-robin`
  - `ip-hash`
  - `cookie-hash`

La estrategia a utilizar para el balanceo de carga. La estrategia que elijas depende de tu carga de trabajo.

#### Round Robin

Round robin es una estrategia de balanceo de carga muy ligera y simple. Simplemente distribuye las solicitudes a los workers de manera rotativa. Esta puede ser una buena estrategia para la mayoría de las cargas de trabajo, es muy simple y tiene muy poca sobrecarga.

**NO** deberías usar round robin si el servidor es con estado (stateful), ya que no garantizará que las solicitudes del mismo cliente se dirijan al mismo worker. Si necesitas un estado persistente, usa IP Hash o Cookie Hash.

Si un worker muere, las solicitudes que se estaban enrutando continuarán hacia el siguiente worker disponible que esté vivo.

#### IP Hash

IP Hash es una estrategia más compleja que garantiza que las solicitudes del mismo cliente se dirijan al mismo worker. Esto es útil para servidores con estado, como las aplicaciones Shiny. IP Hash se aplica en las aplicaciones Shiny si la estrategia se establece en `auto`.

Si un worker muere, las solicitudes se retendrán hasta que el worker vuelva a estar en línea. Esto significa que la latencia puede aumentar si un worker muere.

#### Cookie Hash

Cookie Hash utiliza una cookie para identificar al worker al que se debe enviar la solicitud. Esto es útil para sesiones persistentes (sticky sessions) desde la misma red, incluso si los clientes están detrás de un NAT o comparten la misma dirección IP.

### Type (Tipo de servidor)

- CLI: `--type` o `-t`
- Entorno: `FAUCET_TYPE`
- Predeterminado: `auto`
- Valores posibles:
  - `auto`
  - `plumber`
  - `shiny`
  - `quarto-shiny`
  - `fast-api`

El tipo de servidor a ejecutar. Se utiliza para determinar la estrategia correcta a usar y cómo generar los workers.

#### Auto

Auto intentará determinar el tipo de servidor basándose en el contenido del directorio especificado por `--dir`.

- Si el directorio contiene un archivo `plumber.R` o `entrypoint.R`, se asumirá que es un servidor Plumber.
- Si el directorio contiene un archivo `app.R`, o ambos archivos `server.R` y `ui.R`, se asumirá que es un servidor Shiny.
- Si se proporciona un archivo `.qmd` a través del argumento `--qmd`, o si `FAUCET_QMD` está establecido, se asumirá que es una aplicación Quarto Shiny.
  De lo contrario, faucet saldrá con un error.

#### Plumber

Ejecuta el servidor como una API de Plumber. La estrategia predeterminada es `round-robin`.

#### Shiny

Ejecuta el servidor como una aplicación Shiny. La estrategia predeterminada es `ip-hash`.

#### Quarto Shiny

Ejecuta el servidor como una aplicación Quarto Shiny. La estrategia predeterminada es `ip-hash`. Requiere la opción `--qmd` para especificar el documento Quarto.

#### FastAPI

Ejecuta el servidor como una aplicación FastAPI. La estrategia predeterminada es `round-robin`. Esto requiere que `uv` esté instalado. Faucet buscará un archivo `main.py` en el directorio especificado y lo servirá.

### Directory (Directorio de trabajo)

- CLI: `--dir` o `-d`
- Entorno: `FAUCET_DIR`
- Predeterminado: `.`

El directorio desde el cual ejecutar el servidor. Este debe ser el directorio que contiene el `plumber.R` o el contenido de la aplicación Shiny.

### App Directory (`appDir` de Shiny)

- CLI: `--app-dir` o `-a`
- Entorno: `FAUCET_APP_DIR`
- Predeterminado: `None`

Argumento pasado a `appDir` al ejecutar aplicaciones Shiny. Esto te permite especificar un subdirectorio dentro de la ruta `--dir` como la raíz para la aplicación Shiny.

### QMD (Documento Quarto)

- CLI: `--qmd`
- Entorno: `FAUCET_QMD`
- Predeterminado: `None`

Ruta al archivo `.qmd` de Quarto Shiny. Esto es requerido cuando `type` se establece en `quarto-shiny`, o cuando `type` es `auto` y tienes la intención de ejecutar una aplicación Quarto Shiny.

## Opciones del Subcomando `router`

Estas opciones son específicas del subcomando `router`, utilizado para ejecutar faucet en modo router (experimental).

### Config File

- CLI: `--conf` o `-c`
- Entorno: `FAUCET_ROUTER_CONF`
- Predeterminado: `./frouter.toml`

Ruta al archivo de configuración TOML del router.

## Subcomando `rscript`

Este subcomando te permite ejecutar un script de R arbitrario. Cualquier argumento que siga a `rscript` se pasará directamente al ejecutable `Rscript`.

Ejemplo: `faucet rscript mi_script.R --arg1 valor1`

## Subcomando `uv`

Este subcomando te permite ejecutar comandos `uv` arbitrarios. Esto es particularmente útil para ejecutar scripts de Python o gestionar entornos de Python. Cualquier argumento que siga a `uv` se pasará directamente al ejecutable `uv`.

Ejemplo: `faucet uv run mi_script.py` o `faucet uv pip install pandas`
