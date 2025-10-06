# Preguntas Frecuentes

### faucet no está balanceando la carga de mi aplicación Shiny en Google Cloud Run.

Google Cloud Run tiene un proxy entre las solicitudes enviadas y los
servicios subyacentes reales. Por lo tanto, necesitamos decirle a faucet
quién se está conectando y cómo leer la dirección IP del usuario final.

Podemos solucionarlo configurando la variable de entorno `FAUCET_IP_FROM` o
el argumento CLI `--ip-from` a `x-forwarded-for`.

### Estoy obteniendo errores de "address already in use" con mis workers.

Si ves errores como `createTcpServer: address already in use` o `Failed to create server`, esto típicamente significa que el código de tu aplicación tiene configuraciones de puerto codificadas que entran en conflicto con la gestión de puertos de faucet.

Faucet asigna automáticamente puertos únicos a cada worker, pero el código de tu aplicación podría estar sobrescribiendo estos con declaraciones explícitas de puerto.

**Causas comunes y soluciones:**

- **Aplicaciones Shiny:** Verifica llamadas a `options(shiny.port = ...)` en tu código y elimínalas. También evita puertos codificados en llamadas a `shiny::runApp(port = ...)`.
- **APIs Plumber:** Elimina configuraciones explícitas de puerto en llamadas a `plumber::pr_run(port = ...)`.
- **Otros servicios:** Asegúrate de que no haya puertos codificados en archivos de configuración o scripts de inicio.

Deja que faucet gestione las asignaciones de puerto automáticamente para que el balanceo de carga funcione correctamente.
