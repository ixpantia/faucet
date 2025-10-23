# Opciones / Configuración

Esta sección aborda todas las opciones configurables por el usuario para faucet.

## Opciones Globales

Estas opciones se pueden usar con los subcomandos `start` y `router`.

### Host

- CLI: `--host`
- Entorno: `FAUCET_HOST`
- Por defecto: `127.0.0.1:3838`

La dirección y puerto para vincular el servidor faucet. Si se ejecuta en un
contenedor, esto debería configurarse como `0.0.0.0:3838` para permitir el
acceso externo.

### IP From (Cómo determinar la IP del cliente)

- CLI: `--ip-from` o `-i`
- Entorno: `FAUCET_IP_FROM`
- Por defecto: `client`
- Valores posibles:
    - `client`
    - `x-forwarded-for`
    - `x-real-ip`

Cómo determinar la IP del cliente. Se utiliza para determinar la IP para la
estrategia IP Hash y para el logging de solicitudes HTTP. Si está ejecutando
faucet directamente para usuarios finales, debe utilizar `client`. Si está
ejecutando faucet detrás de un proxy inverso como _nginx_, debe utilizar
`x-forwarded-for` o `x-real-ip`.

> **Nota:** Si está ejecutando faucet detrás de un proxy inverso, asegúrese
> de configurar correctamente el encabezado `X-Forwarded-For` o `X-Real-IP` en la
> configuración de su proxy inverso.

### Rscript (Definir el binario/ejecutable de `Rscript`)

- CLI: `--rscript` o `-r`
- Entorno: `FAUCET_RSCRIPT`
- Por defecto: `Rscript`

Esta opción es útil si tiene varias versiones de R instaladas en su sistema y
necesita especificar una versión específica de `Rscript` para ejecutar su
aplicación. También puede ser útil en plataformas como _Windows_ donde el
ejecutable de `Rscript` no está en el `PATH`.

### Quarto (Definir el binario/ejecutable de `quarto`)

- CLI: `--quarto` o `-q`
- Entorno: `FAUCET_QUARTO`
- Por defecto: `quarto`

El binario/ejecutable de `quarto` a utilizar. Esto es útil si tiene
varias versiones de Quarto instaladas o si no está en su `$PATH`.

### Nivel de Logging (FAUCET_LOG)

- Entorno: `FAUCET_LOG`
- Por defecto: `INFO`
- Valores posibles:
    - `ERROR`
    - `WARN`
    - `INFO`
    - `DEBUG`
    - `TRACE`

El nivel de logging a utilizar. Consulte la sección [logging](./logging.md)
para obtener más información. **Nota:** Esta variable de entorno sigue funcionando, pero
para nuevas aplicaciones generalmente se prefiere configurar el logging mediante un
archivo de configuración del logger o configuraciones específicas de la biblioteca.
Las opciones de CLI `--log-file` y `--max-log-file-size` proporcionan un control más
directo sobre el logging a archivos.

### Log File (Redirigir logging a un archivo)

- CLI: `--log-file` o `-l`
- Entorno: `FAUCET_LOG_FILE`
- Por defecto: `None`

Si utilizas esta varible se deshabilitará el color en la consola y todo el output
será redirigido al archivo especificado. Se añadirá al final del archivo si ya existe.

### Max Log File Size (Tamaño máximo del archivo de log)

- CLI: `--max-log-file-size` o `-m`
- Entorno: `FAUCET_MAX_LOG_FILE_SIZE`
- Por defecto: `None`

El tamaño máximo del archivo de log antes de su rotación (ej. 10M, 1GB).
Requiere que `log-file` esté configurado.

### Shutdown (Apagado)

- CLI: `--shutdown`
- Entorno: `FAUCET_SHUTDOWN`
- Por defecto: `immediate`
- Valores posibles:
    - `immediate`
    - `graceful`

La estrategia que debería utilizar faucet para apagarse. `immediate`
apaga el servidor interrumpiendo cualquier conexión active. `graceful`
espera a que no existan conexiones activas para apagarse.

### Max Message Size (Tamaño máximo del mensaje)

- CLI: `--max-message-size`
- Entorno: `FAUCET_MAX_MESSAGE_SIZE`
- Por defecto: `None`

Tamaño máximo de un mensaje WebSocket. Esto es útil para la prevención de ataques DDOS.
Si no se configura, no hay límite de tamaño.

### Telemetría: Cadena de Conexión PostgreSQL

- CLI: `--pg-con-string`
- Entorno: `FAUCET_TELEMETRY_POSTGRES_STRING`
- Por defecto: `None`

Cadena de conexión a una base de datos PostgreSQL para guardar eventos HTTP. Si se proporciona,
faucet intentará registrar eventos HTTP en esta base de datos.

### Telemetría: Namespace

- CLI: `--telemetry-namespace`
- Entorno: `FAUCET_TELEMETRY_NAMESPACE`
- Por defecto: `faucet`

Namespace bajo el cual se guardan los eventos HTTP en PostgreSQL.

### Telemetría: Versión

- CLI: `--telemetry-version`
- Entorno: `FAUCET_TELEMETRY_VERSION`
- Por defecto: `None`

Representa la versión del código fuente del servicio en ejecución. Esto es útil para
filtrar datos de telemetría.

## Opciones del Subcomando `start`

Estas opciones son específicas del subcomando `start`, utilizado para ejecutar un servidor faucet estándar.

### Workers

- CLI: `--workers` o `-w`
- Entorno: `FAUCET_WORKERS`
- Por defecto: El número de CPUs disponibles para el proceso

La cantidad de procesos de trabajo a crear. En una carga de trabajo ligada a la
CPU, esto debería configurarse al número de CPUs disponibles para el proceso.
En una carga de trabajo ligada a I/O, podría configurarse a un número mayor.

### Strategy (Estrategia)

- CLI: `--strategy` o `-s`
- Entorno: `FAUCET_STRATEGY`
- Por defecto: `round-robin`
- Valores posibles:
    - `round-robin`
    - `ip-hash`
    - `cookie-hash`

La estrategia para el equilibrio de carga. La elección de la estrategia depende
de su carga de trabajo.

#### Round Robin

Round Robin es una estrategia de equilibrio de carga muy ligera y simple.
Distribuye las solicitudes a los trabajadores de manera circular. Puede ser una
buena estrategia para la mayoría de las cargas de trabajo, ya que es muy simple
y tiene muy poco sobrecosto.

**NO** debe usar Round Robin si el servidor es persistente, ya que no
garantizará que las solicitudes del mismo cliente se dirijan al mismo
trabajador. Si necesita un estado persistente, utilice IP Hash o Cookie Hash.

Si un trabajador muere, las solicitudes que se dirigieron a él continuarán al
próximo trabajador disponible que esté vivo.

#### IP Hash

IP Hash es una estrategia más compleja que garantiza que las solicitudes del
mismo cliente se dirijan al mismo trabajador. Esto es útil para servidores
persistentes, como las aplicaciones Shiny. IP Hash se aplica en aplicaciones Shiny
si la estrategia está configurada en `auto`.

Si un trabajador muere, las solicitudes se retendrán hasta que el trabajador
vuelva a estar en línea. Esto significa que la latencia puede aumentar si un
trabajador muere.

#### Cookie Hash

Cookie Hash utiliza una cookie para identificar al trabajador al que se enviará la solicitud.
Esto es útil para sesiones persistentes (sticky sessions) desde la misma red, incluso si los
clientes están detrás de un NAT o comparten la misma dirección IP.

### Type (Tipo de servidor)

- CLI: `--type` o `-t`
- Entorno: `FAUCET_TYPE`
- Por defecto: `auto`
- Valores posibles:
    - `auto`
    - `plumber`
    - `shiny`
    - `quarto-shiny`

El tipo de servidor a ejecutar. Se utiliza para determinar la estrategia
correcta a utilizar y cómo crear los trabajadores.

#### Auto

Auto intentará determinar el tipo de servidor según el contenido del
directorio especificado por `--dir`.
- Si el directorio contiene un archivo `plumber.R` o `entrypoint.R`, se asumirá que es un servidor Plumber.
- Si el directorio contiene un archivo `app.R`, o ambos `server.R` y `ui.R`, se asumirá que es una aplicación Shiny.
- Si se proporciona un archivo `.qmd` a través del argumento `--qmd`, o si `FAUCET_QMD` está configurado, se asumirá que es una aplicación Quarto Shiny.
De lo contrario, faucet terminará con un error.

#### Plumber

Ejecuta el servidor como una API Plumber. La estrategia por defecto es `round-robin`.

#### Shiny

Ejecuta el servidor como una aplicación Shiny. La estrategia por defecto es `ip-hash`.

#### Quarto Shiny

Ejecuta el servidor como una aplicación Quarto Shiny. La estrategia por defecto es `ip-hash`.
Requiere la opción `--qmd` para especificar el documento Quarto.

### Directory (Directorio de trabajo)

- CLI: `--dir` o `-d`
- Entorno: `FAUCET_DIR`
- Por defecto: `.`

El directorio desde el cual ejecutar el servidor. Debería ser el directorio que
contiene el contenido de `plumber.R` o la aplicación Shiny.

### App Directory (`appDir` de Shiny)

- CLI: `--app-dir` o `-a`
- Entorno: `FAUCET_APP_DIR`
- Por defecto: `None`

Argumento pasado como `appDir` al ejecutar aplicaciones Shiny. Esto permite
especificar un subdirectorio dentro de la ruta `--dir` como la raíz para la aplicación Shiny.

### QMD (Documento Quarto)

- CLI: `--qmd`
- Entorno: `FAUCET_QMD`
- Por defecto: `None`

Ruta al archivo `.qmd` de Quarto Shiny. Esto es necesario cuando `type` está configurado
como `quarto-shiny`, o cuando `type` es `auto` y se pretende ejecutar una aplicación Quarto Shiny.

## Opciones del Subcomando `router`

Estas opciones son específicas del subcomando `router`, utilizado para ejecutar faucet en modo router (experimental).

### Config File (Archivo de configuración)

- CLI: `--conf` o `-c`
- Entorno: `FAUCET_ROUTER_CONF`
- Por defecto: `./frouter.toml`

Ruta al archivo de configuración TOML del router.