# Opciones / Configuración

Esta sección aborda todas las opciones configurables por el usuario para
faucet.

## Host

- CLI: `--host`
- Entorno: `FAUCET_HOST`
- Por defecto: `127.0.0.1:3838`

La dirección y puerto para vincular el servidor faucet. Si se ejecuta en un
contenedor, esto debería configurarse como `0.0.0.0:3838` para permitir el
acceso externo.

## Workers

- CLI: `--workers` o `-w`
- Entorno: `FAUCET_WORKERS`
- Por defecto: El número de CPUs disponibles para el proceso

La cantidad de procesos de trabajo a crear. En una carga de trabajo ligada a la
CPU, esto debería configurarse al número de CPUs disponibles para el proceso.
En una carga de trabajo ligada a I/O, podría configurarse a un número mayor.

## Estrategia

- CLI: `--strategy` o `-s`
- Entorno: `FAUCET_STRATEGY`
- Por defecto:
    - Plumber: `round-robin`
    - Shiny: `ip-hash`
- Valores posibles:
    - `round-robin`
    - `ip-hash`

La estrategia para el equilibrio de carga. La elección de la estrategia depende
de su carga de trabajo.

### Round Robin

Round Robin es una estrategia de equilibrio de carga muy ligera y simple.
Distribuye las solicitudes a los trabajadores de manera circular. Puede ser una
buena estrategia para la mayoría de las cargas de trabajo, ya que es muy simple
y tiene muy poco sobrecosto.

**NO** debe usar Round Robin si el servidor es persistente, ya que no
garantizará que las solicitudes del mismo cliente se dirijan al mismo
trabajador. Si necesita un estado persistente, utilice IP Hash.

Si un trabajador muere, las solicitudes que se dirigieron a él continuarán al
próximo trabajador disponible que esté vivo.

### IP Hash

IP Hash es una estrategia más compleja que garantiza que las solicitudes del
mismo cliente se dirijan al mismo trabajador. Esto es útil para servidores
persistentes, como las aplicaciones Shiny.

IP Hash se aplica en aplicaciones Shiny, ya que Round Robin simplemente no
funcionará con ellas.

Si un trabajador muere, las solicitudes se retendrán hasta que el trabajador
vuelva a estar en línea. Esto significa que la latencia puede aumentar si un
trabajador muere.

## Tipo (Tipo de servidor)

- CLI: `--type` o `-t`
- Entorno: `FAUCET_TYPE`
- Por defecto: `auto`
- Valores posibles:
    - `auto`
    - `plumber`
    - `shiny`

El tipo de servidor a ejecutar. Se utiliza para determinar la estrategia
correcta a utilizar y cómo crear los trabajadores.

### Auto

Auto intentará determinar el tipo de servidor según el contenido del
directorio. Si el directorio contiene un archivo `plumber.R`, se asumirá que es
un servidor Plumber. Si el directorio no contiene un archivo `plumber.R`, se
asumirá que es una aplicación Shiny.

### Shiny

Shiny ejecutará el servidor como una aplicación Shiny. Esto utilizará la
estrategia IP Hash.

### Plumber

Plumber ejecutará el servidor como un servidor Plumber. Esto utilizará la
estrategia Round Robin a menos que la opción `--strategy` se establezca en
`ip-hash`.

## Directorio (Directorio de trabajo)

- CLI: `--dir` o `-d`
- Entorno: `FAUCET_DIR`
- Por defecto: `.`

El directorio desde el cual ejecutar el servidor. Debería ser el directorio que
contiene el contenido de `plumber.R` o la aplicación Shiny.

## IP From (Cómo determinar la IP del cliente)

- CLI: `--ip-from`
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

## Logging

- Entorno: `FAUCET_LOG`
- Por defecto: `INFO`
- Valores posibles:
    - `ERROR`
    - `WARN`
    - `INFO`
    - `DEBUG`
    - `TRACE`

El nivel de logging a utilizar. Consulte la sección [logging](./logging.md)
para obtener más información.
