#' Get Session ID
#'
#' Retrieves the Faucet session ID from the Shiny session's request object.
#'
#' @param session The shiny session object. Defaults to `shiny::getDefaultReactiveDomain()`.
#' @return The session ID as a character string.
#' @export
get_session_id <- function(session = shiny::getDefaultReactiveDomain()) {
  request <- session$request
  request$HTTP_FAUCET_SESSION_ID
}

unbox_if_truthy <- function(x) {
  if (is.na(x) || is.null(x) || length(x) == 0) {
    NA
  } else {
    x
  }
}


#' Log an Event
#'
#' This is the core logging function. It constructs a JSON log entry and
#' prints it to stderr. It's not typically called directly, but through wrappers
#' like `info()`, `warn()`, and `error()`.
#'
#' @param level The log level (e.g., "Info", "Warn", "Error").
#' @param message A character string that can be processed by `glue::glue()`.
#' @param ... Arguments to be passed to `glue::glue()` for interpolation into `message`.
#' @param body An optional body for the log event, which can be any R object
#'   that is serializable to JSON. Defaults to `NA_character_`.
#' @param target The target of the log event. Defaults to `"shiny"`.
#' @param event_type The type of the event. Defaults to `"log"`.
#' @param parent The ID of a parent event, if any. Defaults to `NA_character_`.
#' @param .envir The environment in which to evaluate the glue expressions in `message`.
#'
#' @return The generated UUID for the event, as a character string.
#' @export
log <- function(
  level,
  message,
  ...,
  body = NA_character_,
  target = "shiny",
  event_type = "log",
  parent = NA_character_,
  .envir = parent.frame()
) {
  event_id <- uuid::UUIDgenerate()
  message <- glue::glue(message, ..., .envir = .envir)

  contents <- yyjsonr::write_json_str(
    list(
      target = target,
      event_id = event_id,
      parent_event_id = unbox_if_truthy(parent),
      event_type = event_type,
      message = message,
      level = level,
      body = body
    )
  )

  cat(
    sprintf("{{ faucet_event }}: %s\n", contents),
    file = stderr()
  )

  return(event_id)
}

#' Log an Informational Message
#'
#' A wrapper around [log()] for informational messages (level = "Info").
#'
#' @param message A character string that can be processed by `glue::glue()`.
#' @param ... Arguments to be passed to `glue::glue()` for interpolation into `message`.
#' @param body An optional body for the log event. Defaults to `NA_character_`.
#' @param target The target of the log event. Defaults to `"shiny"`.
#' @param event_type The type of the event. Defaults to `"log"`.
#' @param parent The ID of a parent event, if any. Defaults to `NA_character_`.
#' @param .envir The environment in which to evaluate the glue expressions in `message`.
#'
#' @return The generated UUID for the event, as a character string.
#' @seealso [log()]
#' @export
info <- function(
  message,
  ...,
  body = NA_character_,
  target = "shiny",
  event_type = "log",
  parent = NA_character_,
  .envir = parent.frame()
) {
  log(
    level = "Info",
    message = message,
    ...,
    body = body,
    target = target,
    event_type = event_type,
    parent = parent,
    .envir = .envir
  )
}


#' Log a Warning Message
#'
#' A wrapper around [log()] for warning messages (level = "Warn").
#'
#' @param message A character string that can be processed by `glue::glue()`.
#' @param ... Arguments to be passed to `glue::glue()` for interpolation into `message`.
#' @param body An optional body for the log event. Defaults to `NA_character_`.
#' @param target The target of the log event. Defaults to `"shiny"`.
#' @param event_type The type of the event. Defaults to `"log"`.
#' @param parent The ID of a parent event, if any. Defaults to `NA_character_`.
#' @param .envir The environment in which to evaluate the glue expressions in `message`.
#'
#' @return The generated UUID for the event, as a character string.
#' @seealso [log()]
#' @export
warn <- function(
  message,
  ...,
  body = NA_character_,
  target = "shiny",
  event_type = "log",
  parent = NA_character_,
  .envir = parent.frame()
) {
  log(
    level = "Warn",
    message = message,
    ...,
    body = body,
    target = target,
    event_type = event_type,
    parent = parent,
    .envir = .envir
  )
}


#' Log an Error Message
#'
#' A wrapper around [log()] for error messages (level = "Error").
#'
#' @param message A character string that can be processed by `glue::glue()`.
#' @param ... Arguments to be passed to `glue::glue()` for interpolation into `message`.
#' @param body An optional body for the log event. Defaults to `NA_character_`.
#' @param target The target of the log event. Defaults to `"shiny"`.
#' @param event_type The type of the event. Defaults to `"log"`.
#' @param parent The ID of a parent event, if any. Defaults to `NA_character_`.
#' @param .envir The environment in which to evaluate the glue expressions in `message`.
#'
#' @return The generated UUID for the event, as a character string.
#' @seealso [log()]
#' @export
error <- function(
  message,
  ...,
  body = NA_character_,
  target = "shiny",
  event_type = "log",
  parent = NA_character_,
  .envir = parent.frame()
) {
  log(
    level = "Error",
    message = message,
    ...,
    body = body,
    target = target,
    event_type = event_type,
    parent = parent,
    .envir = .envir
  )
}


#' Log a Debug Message
#'
#' A wrapper around [log()] for debug messages (level = "Debug").
#'
#' @param message A character string that can be processed by `glue::glue()`.
#' @param ... Arguments to be passed to `glue::glue()` for interpolation into `message`.
#' @param body An optional body for the log event. Defaults to `NA_character_`.
#' @param target The target of the log event. Defaults to `"shiny"`.
#' @param event_type The type of the event. Defaults to `"log"`.
#' @param parent The ID of a parent event, if any. Defaults to `NA_character_`.
#' @param .envir The environment in which to evaluate the glue expressions in `message`.
#'
#' @return The generated UUID for the event, as a character string.
#' @seealso [log()]
#' @export
debug <- function(
  message,
  ...,
  body = NA_character_,
  target = "shiny",
  event_type = "log",
  parent = NA_character_,
  .envir = parent.frame()
) {
  log(
    level = "Debug",
    message = message,
    ...,
    body = body,
    target = target,
    event_type = event_type,
    parent = parent,
    .envir = .envir
  )
}


#' Log a Trace Message
#'
#' A wrapper around [log()] for trace messages (level = "Trace").
#'
#' @param message A character string that can be processed by `glue::glue()`.
#' @param ... Arguments to be passed to `glue::glue()` for interpolation into `message`.
#' @param body An optional body for the log event. Defaults to `NA_character_`.
#' @param target The target of the log event. Defaults to `"shiny"`.
#' @param event_type The type of the event. Defaults to `"log"`.
#' @param parent The ID of a parent event, if any. Defaults to `NA_character_`.
#' @param .envir The environment in which to evaluate the glue expressions in `message`.
#'
#' @return The generated UUID for the event, as a character string.
#' @seealso [log()]
#' @export
trace <- function(
  message,
  ...,
  body = NA_character_,
  target = "shiny",
  event_type = "log",
  parent = NA_character_,
  .envir = parent.frame()
) {
  log(
    level = "Trace",
    message = message,
    ...,
    body = body,
    target = target,
    event_type = event_type,
    parent = parent,
    .envir = .envir
  )
}
