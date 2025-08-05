library(testthat)

# Mock shiny::getDefaultReactiveDomain() for testing purposes
mock_session <- function(http_faucet_session_id = NULL) {
  list(
    request = list(
      HTTP_FAUCET_SESSION_ID = http_faucet_session_id
    )
  )
}

test_that("get_session_id runs without error", {
  expect_no_error({
    session_with_id <- mock_session(http_faucet_session_id = "test_session_123")
    get_session_id(session = session_with_id)
  })
  expect_no_error({
    session_no_id <- mock_session(http_faucet_session_id = NULL)
    get_session_id(session = session_no_id)
  })
})

test_that("unbox_if_truthy runs without error", {
  expect_no_error(unbox_if_truthy(NA))
  expect_no_error(unbox_if_truthy(NULL))
  expect_no_error(unbox_if_truthy(character(0)))
  expect_no_error(unbox_if_truthy("hello"))
  expect_no_error(unbox_if_truthy(123))
  expect_no_error(unbox_if_truthy(TRUE))
})

test_that("log function runs without error", {
  # Temporarily redirect stderr to avoid polluting test output with log messages
  expect_no_error({
    log(level = "Info", message = "Test message")
    log(level = "Info", message = "Test with body", body = list(data = 123))
    name <- "World"
    log(level = "Info", message = "Hello {name}")
  })
})

test_that("info, warn, error, debug, trace wrappers run without error", {
  expect_no_error(info(message = "Info message"))
  expect_no_error(warn(message = "Warn message"))
  expect_no_error(error(message = "Error message"))
  expect_no_error(debug(message = "Debug message"))
  expect_no_error(trace(message = "Trace message"))
})
