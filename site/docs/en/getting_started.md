## Quick Start

To use Faucet, ensure that it is installed. If not, refer to the [official installation documentation](#link-to-installation-docs).

Once installed, use the following command to start Faucet with default settings:

```bash
# Start Faucet
faucet
```

Faucet will bind to `127.0.0.1:3838` and automatically determine the number of worker threads based on the number of CPUs on the host machine.

## Running a Shiny Application

Let's create a simple Shiny application and deploy it using Faucet.

1. Create a basic Shiny app named `app.R`:

```R
# app.R
library(shiny)

ui <- fluidPage(
  shinyOutput("hello")
)

server <- function(input, output) {
  output$hello <- renderText({
    "Hello, Faucet!"
  })
}

shinyApp(ui, server)
```

2. Save the above code in a file named `app.R`.

3. Start Faucet in the same directory as your Shiny app:

```bash
faucet
```

Faucet will automatically detect the Shiny app and deploy it.

4. Open your web browser and navigate to [http://127.0.0.1:3838](http://127.0.0.1:3838) to see your Shiny app in action.


### Adding more workers

If your computer has more than one CPU core, then you probably saw that
many workers were created when you started Faucet. This is because Faucet
automatically detects the number of CPU cores on your computer and creates
a worker for each core.

You can customize the number of workers by using the `--workers` flag:

```bash
faucet --workers 4
```

Or by setting the `FAUCET_WORKERS` environment variable:

```bash
export FAUCET_WORKERS=4
faucet
```

In both cases, Faucet will create 4 workers on random available ports.
Traffic will be load balanced across all workers according to the
IP address of the incoming request. This means that if you have 4 workers,
then you can handle 4 times as many concurrent requests as a single worker.

## Conclusion

Congratulations! You have successfully started using Faucet and deployed a
basic Shiny application with many workers.

Happy coding with Faucet!
