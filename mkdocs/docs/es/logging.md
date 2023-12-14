# Logging

faucet se encarga de hacer logs tanto de las solicitudes y respuestas HTTP como
de la salida estándar (`stdout`) y la salida de error (`stderr`) de los
procesos trabajadores. Esta sección describe cómo funcionan los logs en faucet
y cómo filtrar los logs.

## Estructura básica

Todos los logs generados por faucet siguen la siguiente estructura:

```
[<marca de tiempo> nivel <fuente>] <mensaje>
```

 - La marca de tiempo tiene el formato `AAAA-MM-DDTHH:MM:SSZ` y está en UTC.
 - El nivel puede ser uno de los siguientes:
    - `ERROR`
    - `WARN`
    - `INFO`
    - `DEBUG`
    - `TRACE`
 - La fuente es ya sea faucet o el nombre del trabajador `Worker::<id>`.

## Logging HTTP

Los logs HTTP se registran todos a nivel `INFO`. La fuente es el trabajador
encargado de manejar la solicitud. El mensaje tiene la siguiente forma:

```
<ip> "<método> <ruta> <protocolo>" <estado> "<agente-de-usuario>" <duración>
```

 - `ip` es la dirección IP del cliente (determinada por la opción `--ip-from`).
 - `método` es el método HTTP utilizado.
 - `ruta` es la ruta de la solicitud.
 - `protocolo` es la versión del protocolo HTTP utilizada.
 - `estado` es el código de estado HTTP devuelto.
 - `agente-de-usuario` es el agente de usuario del cliente.
 - `duración` es el tiempo que tomó manejar la solicitud en milisegundos.

## Logging de trabajadores

Los logs de trabajadores se dividen en dos componentes: `stdout` y `stderr`.
`stdout` se loggea a nivel `INFO` y `stderr` se loggea a nivel `WARN`. La
fuente es el trabajador que posee el proceso subyacente. El mensaje es la línea
de salida del proceso.

## Filtrado de logs

Por defecto, faucet logea a nivel `INFO`, lo que significa que se muestran los
logs de `ERROR`, `WARN` e `INFO`. Para cambiar el nivel de log, utilice la
variable de entorno `FAUCET_LOG`.

> **Nota:** Plumber imprime errores que ocurren en puntos finales en `stdout`,
> por lo que si desea ver esos errores, deberá establecer el nivel de log en
> `INFO` o inferior. Shiny, por otro lado, imprime errores en `stderr`, por lo
> que deberá establecer el nivel de log en `WARN` o inferior para ver esos
> errores.
