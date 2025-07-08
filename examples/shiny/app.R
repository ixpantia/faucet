library(shiny)
library(bslib)

inject <- paste(
  tags$script(src='/reconnect.js'),
  HTML("</head>"),
  sep="\n"
)

filter <- function(...) {
  # The signature of filter functions changed between Shiny 0.4.0 and
  # 0.4.1; previously the parameters were (ws, headers, response) and
  # after they became (request, response). To work with both types, we
  # just grab the last argument.
  response <- list(...)[[length(list(...))]]

  if (response$status < 200 || response$status > 300) return(response)

  # Don't break responses that use httpuv's file-based bodies.
  if ('file' %in% names(response$content))
     return(response)
                                            
  if (!grepl("^text/html\\b", response$content_type, perl=T))
     return(response)

  # HTML files served from static handler are raw. Convert to char so we
  # can inject our head content.
  if (is.raw(response$content))
     response$content <- rawToChar(response$content)

  response$content <- sub("</head>", inject, response$content, 
     ignore.case=T)
  return(response)
}
options(shiny.http.response.filter=filter)

# Define UI for app that draws a histogram ----
ui <- page_sidebar(
  # App title ----
  title = "Hello Shiny!",
  tags$head(
    tags$script("console.log('Hello Shiny!');")
  ),
  # Sidebar panel for inputs ----
  sidebar = sidebar(
    # Input: Slider for the number of bins ----
    sliderInput(
      inputId = "bins",
      label = "Number of bins:",
      min = 1,
      max = 50,
      value = 30
    )
  ),
  # Output: Histogram ----
  plotOutput(outputId = "distPlot")
)

# Define server logic required to draw a histogram ----
server <- function(input, output) {

  # Histogram of the Old Faithful Geyser Data ----
  # with requested number of bins
  # This expression that generates a histogram is wrapped in a call
  # to renderPlot to indicate that:
  #
  # 1. It is "reactive" and therefore should be automatically
  #    re-executed when inputs (input$bins) change
  # 2. Its output type is a plot
  output$distPlot <- renderPlot({

    x    <- faithful$waiting
    bins <- seq(min(x), max(x), length.out = input$bins + 1)

    hist(x, breaks = bins, col = "#007bc2", border = "white",
         xlab = "Waiting time to next eruption (in mins)",
         main = "Histogram of waiting times")

    })

}

shinyApp(ui, server)
