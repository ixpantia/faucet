# Telemetría en Faucet

Faucet incluye una función de telemetría diseñada para ayudarte a monitorear el rendimiento y los patrones de uso de tus aplicaciones desplegadas. Cuando está habilitada, Faucet puede enviar datos de telemetría a una base de datos PostgreSQL, permitiendo el análisis y la obtención de información sobre cómo están operando tus instancias de Faucet y las aplicaciones R subyacentes.

Este documento describe cómo configurar y utilizar las capacidades de telemetría de Faucet.

## Configuración de la Base de Datos

Antes de habilitar la telemetría, necesitas configurar tu base de datos PostgreSQL con la tabla requerida. Faucet enviará sus datos de telemetría a una tabla llamada `faucet_http_events`.

Puedes crear esta tabla usando el siguiente comando SQL:

```sql
CREATE TABLE faucet_http_events (
    request_uuid UUID,
    namespace TEXT,
    version TEXT,
    target TEXT,
    worker_route TEXT,
    worker_id INT,
    ip_addr INET,
    method TEXT,
    path TEXT,
    query_params TEXT,
    http_version TEXT,
    status SMALLINT,
    user_agent TEXT,
    elapsed BIGINT,
    time TIMESTAMPTZ NOT NULL
);
```

**Nota para Usuarios de TimescaleDB:**

Si estás utilizando TimescaleDB, opcionalmente puedes convertir esta tabla en una hypertable para una mejor gestión de datos de series temporales. Después de crear la tabla como se muestra arriba, puedes ejecutar el siguiente comando SQL:

```sql
SELECT create_hypertable('faucet_http_events', by_range('time'));
```
Este paso es específico para TimescaleDB y mejora sus capacidades para manejar grandes volúmenes de datos de series temporales.

## Habilitación y Configuración de la Telemetría

La telemetría en Faucet está deshabilitada por defecto. Para habilitarla, debes proporcionar una cadena de conexión de PostgreSQL. La configuración se puede realizar mediante opciones de línea de comandos o variables de entorno.

### Opciones Clave de Configuración:

1.  **Cadena de Conexión de PostgreSQL:**
    *   **CLI:** `--telemetry-postgres-string <CADENA_DE_CONEXION>`
    *   **Variable de Entorno:** `FAUCET_TELEMETRY_POSTGRES_STRING=<CADENA_DE_CONEXION>`
    *   **Descripción:** Esta es la configuración esencial para habilitar la telemetría. La cadena de conexión debe estar en un formato adecuado para conectarse a tu base de datos PostgreSQL (por ejemplo, `postgresql://usuario:contraseña@host:puerto/basededatos`). Faucet utilizará esto para enviar datos de telemetría.
    *   **Por Defecto:** `None` (Telemetría deshabilitada)

2.  **Espacio de Nombres de Telemetría (Namespace):**
    *   **CLI:** `--telemetry-namespace <NAMESPACE>`
    *   **Variable de Entorno:** `FAUCET_TELEMETRY_NAMESPACE=<NAMESPACE>`
    *   **Descripción:** Te permite definir un espacio de nombres para los datos de telemetría. Esto es útil si estás recopilando datos de múltiples instancias de Faucet o diferentes servicios en la misma base de datos, ayudando a segmentar e identificar la fuente de los datos.
    *   **Por Defecto:** `faucet`

3.  **Versión de Telemetría:**
    *   **CLI:** `--telemetry-version <VERSION>`
    *   **Variable de Entorno:** `FAUCET_TELEMETRY_VERSION=<VERSION>`
    *   **Descripción:** Especifica la versión del servicio o aplicación que está siendo ejecutada/monitoreada por Faucet. Puede ser la versión de tu aplicación o la versión de Faucet misma. Es útil para filtrar datos de telemetría y correlacionar observaciones con despliegues específicos.
    *   **Por Defecto:** `None`

Para más detalles sobre estas opciones, consulta la página de [Opciones de Línea de Comandos](./options.md).

## Datos Recopilados

El sistema de telemetría de Faucet está diseñado para capturar información relevante para los aspectos operativos del servidor y las aplicaciones que gestiona. Aunque el esquema exacto y los puntos de datos pueden evolucionar, las categorías generales de datos recopilados incluyen:

*   **Métricas de Solicitud/Respuesta:** Información sobre las solicitudes HTTP entrantes y las respuestas generadas, como rutas de solicitud, códigos de estado de respuesta y latencias.
*   **Rendimiento del Worker:** Datos relacionados con el comportamiento de los procesos worker individuales, incluyendo potencialmente tiempos de procesamiento y tasas de error.
*   **Eventos de Balanceo de Carga:** Información sobre cómo se distribuyen las solicitudes si se utilizan estrategias de balanceo de carga.
*   **Información de la Instancia:** Detalles como el espacio de nombres y la versión configurados, para ayudar a contextualizar los datos.

Los datos se estructuran para ser almacenados en una base de datos PostgreSQL, lo que permite consultas basadas en SQL e integración con diversas herramientas de análisis y visualización.

## Utilización de los Datos de Telemetría

Una vez que la telemetría está configurada y Faucet está enviando datos a tu base de datos PostgreSQL, puedes:

*   **Monitorear la Salud de la Aplicación:** Rastrear tasas de error, tiempos de respuesta y otros indicadores clave de rendimiento (KPIs) para asegurar que tus aplicaciones funcionen sin problemas.
*   **Entender Patrones de Uso:** Analizar volúmenes de solicitudes, puntos finales populares y actividad del usuario para obtener información sobre cómo se están utilizando tus aplicaciones.
*   **Solucionar Problemas:** Correlacionar datos de telemetría con registros y otras herramientas de monitoreo para diagnosticar y resolver problemas de manera más efectiva.
*   **Planificación de Capacidad:** Observar la utilización de recursos y las tendencias de rendimiento a lo largo del tiempo para tomar decisiones informadas sobre el escalado de tu infraestructura.
*   **Optimización del Rendimiento:** Identificar cuellos de botella u operaciones lentas examinando las latencias de las solicitudes y los datos de rendimiento de los workers.

Puedes conectarte a la base de datos PostgreSQL utilizando clientes SQL estándar, herramientas de inteligencia de negocios o scripts personalizados para consultar y visualizar los datos de telemetría recopilados según tus necesidades.