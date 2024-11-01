shinyServer(function(input, output) {
  output$distPlot <- renderPlotly({
    # Generate a normal distribution based on input$num
    data <- data.frame(x = rnorm(input$num))

    # Create a ggplot histogram
    p <- ggplot(data, aes(x = x)) +
      geom_histogram(binwidth = 0.2, fill = "steelblue", color = "white") +
      theme_minimal() +
      labs(
        title = "Histogram of Normally Distributed Data",
        x = "Value",
        y = "Frequency"
      )

    # Convert ggplot to an interactive Plotly plot
    ggplotly(p)
  })
})
