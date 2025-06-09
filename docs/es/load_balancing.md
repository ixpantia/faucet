# Estrategias de Balanceo de Carga en Faucet

El balanceo de carga es un componente crítico para distribuir el tráfico de red entre múltiples procesos de trabajo (workers) de Faucet, cada uno ejecutando típicamente una instancia de una aplicación R (como Shiny, Plumber o Quarto Shiny). Esta distribución asegura que ningún trabajador individual se sobrecargue, lo que conduce a una mejor capacidad de respuesta, disponibilidad y confiabilidad de sus aplicaciones desplegadas. Este documento describe las estrategias de balanceo de carga disponibles en Faucet, sus casos de uso y sus respectivas ventajas y desventajas.

Faucet le permite configurar la estrategia de balanceo de carga utilizando la opción de línea de comandos `--strategy` o la variable de entorno `FAUCET_STRATEGY`. Las estrategias disponibles son:

*   Round Robin
*   IP Hash
*   Cookie Hash

## Estrategias Predeterminadas

Faucet aplica estrategias predeterminadas basadas en el tipo de aplicación if no se especifica explícitamente:
*   **Aplicaciones Shiny y Quarto Shiny:** Predeterminan `ip-hash` para asegurar la persistencia de la sesión.
*   **APIs de Plumber:** Predeterminan `round-robin` ya que a menudo son sin estado.

## Características Comunes: Salud del Trabajador y Reintentos

Todas las estrategias de balanceo de carga en Faucet incorporan un mecanismo para manejar procesos de trabajo fuera de línea. Si se detecta que un trabajador backend seleccionado está fuera de línea:

1.  Faucet registrará el intento de conexión al trabajador fuera de línea.
2.  Se emplea un mecanismo de reintento con retroceso exponencial (exponential backoff).
3.  **El comportamiento ante el fallo de un trabajador difiere según la estrategia:**
    *   **Round Robin:** Después de una corta espera (`WAIT_TIME_UNTIL_RETRY`), Faucet intentará enrutar la solicitud al *siguiente trabajador disponible* en la secuencia.
    *   **IP Hash & Cookie Hash:** Faucet continuará reintentando la conexión con el *trabajador designado originalmente*. Esto significa que las solicitudes para ese trabajador específico quedan efectivamente "retenidas" y experimentarán latencia o parecerán colgarse hasta que el trabajador vuelva a estar en línea o la solicitud expire. Los clientes no se redirigen automáticamente a un trabajador diferente porque eso rompería la persistencia de la sesión.

## 1. Round Robin

### Descripción
La estrategia Round Robin distribuye las solicitudes entrantes a los procesos de trabajo de Faucet en un orden secuencial. Cada nueva solicitud se envía al siguiente trabajador de la lista. Cuando se llega al final de la lista, el balanceador de carga vuelve al principio y comienza de nuevo.

### Casos de Uso
*   **Aplicaciones sin Estado (Stateless):** Ideal para aplicaciones sin estado como muchas APIs de Plumber, donde cada solicitud puede ser manejada independientemente por cualquier trabajador.
*   **Despliegues Simples:** Adecuado cuando se espera que todos los procesos de trabajo tengan capacidades de procesamiento similares.

### Ventajas
*   **Simplicidad:** Fácil de entender e implementar.
*   **Distribución Equitativa (condiciones ideales):** Si todos los trabajadores están saludables y tienen capacidades similares, Round Robin puede distribuir el tráfico de manera relativamente uniforme.
*   **Baja Sobrecarga:** Costo computacional mínimo para el balanceador de carga.
*   **Resiliencia ante Fallos de Trabajadores:** Si un trabajador se desconecta, las solicitudes se enrutan automáticamente al siguiente trabajador disponible después de un breve retraso.

### Desventajas
*   **Ignora la Carga del Trabajador:** No tiene en cuenta la carga actual en los procesos de trabajo individuales (más allá de las verificaciones básicas de en línea/fuera de línea).
*   **Sin Persistencia de Sesión:** Los clientes pueden ser dirigidos a diferentes trabajadores en solicitudes posteriores. Esto lo hace **inadecuado** para aplicaciones con estado como Shiny o Quarto Shiny que requieren afinidad de sesión (por ejemplo, mantener datos específicos del usuario o estados de entrada).
*   **Distribución Desigual con Capacidades Variables:** Si los procesos de trabajo tienen diferentes capacidades subyacentes (aunque Faucet típicamente genera procesos R idénticos), algunos podrían sobrecargarse.

## 2. IP Hash

### Descripción
La estrategia IP Hash utiliza la dirección IP del cliente para determinar qué proceso de trabajo de Faucet manejará la solicitud. Se aplica una función hash a la dirección IP del cliente y el valor hash resultante se asigna consistentemente a un trabajador específico.

**Importante para Configuraciones con Proxy Inverso:** Si Faucet se ejecuta detrás de un proxy inverso (por ejemplo, Nginx, Apache), es crucial configurar correctamente la opción `--ip-from` (o la variable de entorno `FAUCET_IP_FROM`). Esto indica a Faucet si debe usar la IP directa del cliente o una IP de una cabecera como `X-Forwarded-For` o `X-Real-IP`, asegurando una identificación de IP precisa para esta estrategia.

### Casos de Uso
*   **Aplicaciones con Estado (Predeterminado para Shiny/Quarto):** Esencial para aplicaciones como Shiny y Quarto Shiny que requieren persistencia de sesión. Asegura que un cliente sea dirigido consistentemente al mismo proceso de trabajo, manteniendo el estado de su sesión.
*   **Beneficios de Caché:** Puede mejorar las tasas de aciertos de caché en el trabajador si los datos se almacenan en caché según las interacciones del usuario.

### Ventajas
*   **Persistencia de Sesión:** Garantiza que las solicitudes de la misma IP de cliente se dirijan consistentemente al mismo trabajador, crucial para las aplicaciones R con estado.
*   **Enrutamiento Determinista:** La misma IP siempre se enrutará al mismo trabajador (suponiendo que el grupo de trabajadores no haya cambiado).

### Desventajas
*   **Distribución de Carga Desigual:**
    *   Si unas pocas direcciones IP generan un volumen de tráfico desproporcionadamente grande, los trabajadores asignados a esas IP pueden sobrecargarse.
    *   Los clientes detrás de una puerta de enlace de Traducción de Direcciones de Red (NAT) o un gran proxy corporativo parecerán tener todos la misma IP de origen. Todos estos clientes serán dirigidos al mismo trabajador, lo que podría sobrecargarlo.
*   **Cambio de IP del Cliente:** La persistencia de la sesión puede perderse si la dirección IP de un cliente cambia durante su sesión (por ejemplo, usuarios móviles cambiando entre Wi-Fi y datos celulares).
*   **Fallos del Trabajador:** Si un trabajador designado se cae, las solicitudes de los clientes cuyo hash de IP corresponde a ese trabajador serán retenidas y reintentadas contra el *mismo* trabajador, lo que provocará retrasos para esos usuarios hasta que el trabajador se restaure. No se redirigen automáticamente para preservar la integridad de la sesión.

## 3. Cookie Hash

### Descripción
La estrategia Cookie Hash logra la persistencia de la sesión mediante el uso de una cookie HTTP llamada `FAUCET_LB_COOKIE`. Cuando llega una solicitud:
1.  Faucet verifica la existencia de la cookie `FAUCET_LB_COOKIE`.
2.  Si la cookie existe y contiene un UUID válido, Faucet utiliza este UUID para seleccionar consistentemente un proceso de trabajo backend.
3.  Si la cookie no está presente, no es válida, o si la estrategia es `CookieHash` y no se encuentra un UUID de cookie adecuado, **Faucet genera un nuevo UUID**.
4.  Este UUID (ya sea extraído o recién generado) se utiliza para determinar el trabajador.
5.  De manera crucial, Faucet **establecerá (o actualizará) la `FAUCET_LB_COOKIE` en la respuesta HTTP**, incluyendo el UUID. Esto asegura que las solicitudes posteriores del mismo navegador cliente incluyan esta cookie, dirigiéndolas al mismo trabajador.

Este mecanismo asegura que el cliente sea dirigido consistentemente al mismo trabajador para solicitudes posteriores, siempre y cuando su navegador acepte y envíe cookies.

### Casos de Uso
*   **Aplicaciones Robustas con Estado:** Proporciona persistencia de sesión confiable para Shiny, Quarto Shiny u otras aplicaciones con estado. Es particularmente beneficioso cuando las direcciones IP de los clientes no son estables o cuando muchos clientes pueden compartir una dirección IP (por ejemplo, usuarios detrás de grandes NATs o proxies).
*   **Control Fino de Sesiones:** Ofrece un control más preciso sobre la afinidad de sesión que IP Hash, ya que se basa en un identificador único (el UUID de la cookie) específico de la sesión del cliente, gestionado por Faucet.

### Ventajas
*   **Persistencia de Sesión Confiable:** Más robusto que IP Hash en escenarios con IPs de cliente dinámicas o NAT, ya que depende de la cookie gestionada por Faucet.
*   **Mejor Distribución de Carga (que IP Hash en escenarios NAT):** Puede distribuir la carga de manera más uniforme que IP Hash cuando muchos usuarios comparten la misma IP de origen, ya que la sesión del navegador de cada usuario obtendrá su propia `FAUCET_LB_COOKIE` con un UUID único.
*   **Enrutamiento Determinista:** El mismo UUID de cookie se enrutará consistentemente al mismo trabajador (suponiendo que el grupo de trabajadores sea estable).
*   **Gestión Automática de Cookies por Faucet:** Faucet maneja la generación y el establecimiento de la cookie necesaria, simplificando la configuración.

### Desventajas
*   **Soporte de Cookies del Cliente:** Depende de que los clientes acepten y envíen cookies. Si un cliente tiene las cookies deshabilitadas, esta estrategia no proporcionará persistencia de sesión.
*   **Sobrecarga de Cookies:** Implica la sobrecarga estándar de transmisión y procesamiento de cookies HTTP, aunque la gestión de Faucet es eficiente.
*   **Fallos del Trabajador:** Similar a IP Hash, si un trabajador designado por un hash de cookie se cae, las solicitudes asociadas con ese hash de cookie serán retenidas y reintentadas contra el *mismo* trabajador, causando potencialmente retrasos a los usuarios afectados.
*   **Solicitudes Simultáneas Iniciales:** Como se indica en el código fuente de Faucet, si un navegador envía múltiples solicitudes iniciales *simultáneas* antes de que la primera respuesta `Set-Cookie` sea procesada y devuelta por el navegador, esas solicitudes iniciales podrían brevemente acceder a diferentes trabajadores antes de establecerse en el determinado por la cookie finalmente establecida. Este es un caso límite menor para la mayoría de las aplicaciones.

---

Elegir la estrategia de balanceo de carga correcta en Faucet depende en gran medida de los requisitos específicos de su aplicación R, particularmente su estado (statefulness), y su entorno de despliegue (por ejemplo, independiente vs. detrás de un proxy inverso). Para aplicaciones Shiny y Quarto Shiny, generalmente se recomienda `ip-hash` (predeterminado) o `cookie-hash`. Para APIs de Plumber sin estado, `round-robin` (predeterminado) suele ser suficiente.