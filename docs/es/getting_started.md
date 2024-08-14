## Inicio Rápido

Para usar faucet, asegúrate de que esté instalado. Si no lo está, consulta la [documentación oficial de instalación](./install.md).

Una vez instalado, utiliza el siguiente comando para iniciar faucet con la configuración predeterminada:

```bash
# Iniciar faucet
faucet start
```

faucet se vinculará a `127.0.0.1:3838` y determinará automáticamente el número de hilos de trabajo según el número de CPU en la máquina host.

## Ejecutar una Aplicación Shiny

Creemos una aplicación Shiny simple y despliégala usando faucet.

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

4. Abre tu navegador web y dirígete a [http://127.0.0.1:3838](http://127.0.0.1:3838) para ver tu aplicación Shiny en acción.

### Añadir más trabajadores

Si tu computadora tiene más de un núcleo de CPU, probablemente hayas visto que se crearon muchos trabajadores al iniciar faucet. Esto se debe a que faucet detecta automáticamente el número de núcleos de CPU en tu computadora y crea un trabajador por cada núcleo.

Puedes personalizar el número de trabajadores utilizando la bandera `--workers`:

```bash
faucet start --workers 4
```

O configurando la variable de entorno `FAUCET_WORKERS`:

```bash
export FAUCET_WORKERS=4
faucet start
```

En ambos casos, faucet creará 4 trabajadores en puertos aleatorios disponibles. El tráfico se equilibrará entre todos los trabajadores según la dirección IP de la solicitud entrante. Esto significa que si tienes 4 trabajadores, podrás manejar 4 veces más solicitudes concurrentes que un solo trabajador.

## Conclusión

¡Felicidades! Has comenzado a usar faucet y desplegado una aplicación Shiny básica con muchos trabajadores.

¡Feliz programación con faucet!
