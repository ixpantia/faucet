shinyUI(fluidPage(
  theme = bs_theme(), # Apply default bslib theme

  # Application title
  titlePanel("Simple Shiny App with Plotly and bslib"),

  # Sidebar layout with input and output definitions
  sidebarLayout(
    sidebarPanel(
      sliderInput("num",
        "Number of Observations:",
        min = 10,
        max = 1000,
        value = 500
      )
    ),

    # Main panel for displaying outputs
    mainPanel(
      plotlyOutput("distPlot")
    )
  )
))
