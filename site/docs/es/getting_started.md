## Inicio Rápido

Para usar Faucet, asegúrate de que esté instalado. Si no lo está, consulta la [documentación oficial de instalación](#link-to-installation-docs).

Una vez instalado, utiliza el siguiente comando para iniciar Faucet con la configuración predeterminada:

```bash
# Iniciar Faucet
faucet
```

Faucet se vinculará a `127.0.0.1:3838` y determinará automáticamente el número de hilos de trabajo según el número de CPU en la máquina host.

## Ejecutar una Aplicación Shiny

Creemos una aplicación Shiny simple y despliégala usando Faucet.

1. Crea una aplicación Shiny básica llamada `app.R`:

```R
# app.R
library(shiny)

ui <- fluidPage(
  shinyOutput("hello")
)

server <- function(input, output) {
  output$hello <- renderText({
    "¡Hola, Faucet!"
  })
}

shinyApp(ui, server)
```

2. Guarda el código anterior en un archivo llamado `app.R`.

3. Inicia Faucet en el mismo directorio que tu aplicación Shiny:

```bash
faucet
```

Faucet detectará automáticamente la aplicación Shiny y la desplegará.

4. Abre tu navegador web y dirígete a [http://127.0.0.1:3838](http://127.0.0.1:3838) para ver tu aplicación Shiny en acción.

### Añadir más trabajadores

Si tu computadora tiene más de un núcleo de CPU, probablemente hayas visto que se crearon muchos trabajadores al iniciar Faucet. Esto se debe a que Faucet detecta automáticamente el número de núcleos de CPU en tu computadora y crea un trabajador por cada núcleo.

Puedes personalizar el número de trabajadores utilizando la bandera `--workers`:

```bash
faucet --workers 4
```

O configurando la variable de entorno `FAUCET_WORKERS`:

```bash
export FAUCET_WORKERS=4
faucet
```

En ambos casos, Faucet creará 4 trabajadores en puertos aleatorios disponibles. El tráfico se equilibrará entre todos los trabajadores según la dirección IP de la solicitud entrante. Esto significa que si tienes 4 trabajadores, podrás manejar 4 veces más solicitudes concurrentes que un solo trabajador.

## Conclusión

¡Felicidades! Has comenzado a usar Faucet y desplegado una aplicación Shiny básica con muchos trabajadores.

¡Feliz programación con Faucet!
