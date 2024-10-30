# plumber.R

#* @param n Seconds to sleep
#* @get /sleep
function(n = 1L) {
  Sys.sleep(as.numeric(n))
}

#* Echo the parameter that was sent in
#* @param msg The message to echo back.
#* @get /echo
function(msg = "") {
  list(msg = paste0("The message is: '", msg, "'"))
}

#* Plot out data from the iris dataset
#* @param spec If provided, filter the data to only this species (e.g. 'setosa')
#* @get /plot
#* @serializer png
function(spec) {
  myData <- iris
  title <- "All Species"

  # Filter if the species was specified
  if (!missing(spec)) {
    title <- paste0("Only the '", spec, "' Species")
    myData <- subset(iris, Species == spec)
  }

  plot(myData$Sepal.Length, myData$Petal.Length,
    main = title, xlab = "Sepal Length", ylab = "Petal Length"
  )
}
