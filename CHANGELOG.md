## [2.1.1] - 2026-06-23

### 🐛 Bug Fixes

- Fixes bug with httpuv and multiple cookie headers (#275)
## [2.1.0] - 2025-10-23

### 🚀 Features

- Add PostgreSQL SSL envvars and command-line options (#254)
- Adds new FAUCET_MAX_MESSAGE_SIZE option (#260)

### 📚 Documentation

- Update FAQs for Address/Port already in use issue (#252)
## [2.0.1] - 2025-09-02

### 🐛 Bug Fixes

- Fixes query param detection in case insensitive form (#245)
## [2.0.0] - 2025-09-01

### 🚀 Features

- Implement auto-scaling for Plumber APIs based on RPS (#209)
- Adds reconnect logic for Shiny sessions (#217)
- Adds logging mechanism to use within applications (#229)

### 🐛 Bug Fixes

- Fixes bug with double reconnection (#224)
- Fixes port reuse by different workers (#237)

### 📚 Documentation

- Improves documentation and error messages (#243)

### ⚙️ Miscellaneous Tasks

- Updates tokio-tungstenite
- Adds levels to logging
- Adds tests to r package
- Runs devtools document
- Update docker images to use latest versions
- Updates cross.sh
## [1.2.0] - 2025-06-10

### 🚀 Features

- Takes care of child process cleaup
- Allows saving of HTTP events into PostgreSQL (#150)
- Adds log file rotation and limit
- Reads Renviron on root of Working Dir
- Adds image build for R4.5 (#191)
- Implements cookie based load balancing (#193)

### 🐛 Bug Fixes

- Adds half close and better debug
- Fixes port assign range for plumber

### 📚 Documentation

- Adds examples for use in docker
- Adds documentation for cookie hash
- Adds documentation for router

### ⚙️ Miscellaneous Tasks

- Improves from clippy suggestions
- Waits for kill processes to exit
- Cargo update
- Updates dockerfiles and github workflows
- Update dependencies
- Update dependencies
## [1.1.0] - 2024-10-03

### 🚀 Features

- Adds graceful shutdown on Unix platforms

### 📚 Documentation

- Documents shutdown strategies

### ⚙️ Miscellaneous Tasks

- Cargo clippy
- Adds CLI arg for shutdown strategy
- Fixes release actions
## [1.0.3] - 2024-09-16

### ⚙️ Miscellaneous Tasks

- Bumps version to 1.0.3
## [1.0.2] - 2024-09-16

### ⚙️ Miscellaneous Tasks

- Improves performance by removing async-trait
- Updates hash function for ip addresses
- Removes allocation for SEC WebSockets
- Bumps version to v1.0.2
- Updates cargo lock
## [1.0.0] - 2024-09-14

### 🚀 Features

- Implements experimental router
- Adds support for Quarto Shiny files
- Allows log redirection to a file

### 🐛 Bug Fixes

- Fixes panic on windows with Ctrl-C
- Fixes error with absolute paths
- Fixes naming conflict for CLI arguments

### 📚 Documentation

- Documents the use of `faucet start`
- Creates a contributing.md document
- Documents use of log redirection
- Documents load balancing in google cloud run
- Updates version in documentation
- Documents the use of the router and qmd
- Changes version to 1.0.0 in README
- Adds context to readme

### ⚙️ Miscellaneous Tasks

- Changes WorkerState structure
- Unifies services into static allocation
- Cargo fmt + Cargo clippy
- Minor fixes
- Updates dependencies
- Improves prefix matching
- Minor CLI improvement
- Add nightly workflow
- Add workflow dispatch to nightly
- Remove print
- Updates dockerfile
- Handles connection exit error
- Removes required app_dir arg in router
- Bump version to 0.7
- Fixes unit test error
- Cargo fix
- Cargo update
- Updates dependencies and sem ver
## [0.6.0] - 2024-06-21

### 🚀 Features

- Adds FAUCET_WORKER_ID environment variable

### 📚 Documentation

- Fixes download links on documentation

### ⚙️ Miscellaneous Tasks

- Fix documentation build fail
- Updates dependencies
- Updates Cargo.toml
- Updates base64 to 0.22
- Changes tower version
- Removes minimal version testing
- Adds workflows for R 4.4
- Bump version to 0.6
- Updates lock file
## [0.5.2] - 2024-01-16

### 🚀 Features

- Reserves ports to prevent possible collisions

### 🐛 Bug Fixes

- Improves Rscript command usage on Windows

### 📚 Documentation

- Documents the use of faucet with reverse proxies

### ⚙️ Miscellaneous Tasks

- Moves documentation to root of repo
- Changes CI to always publish gh pages on main
- Fixes readme
- Removes unnecessary version annotations
- Adds script to cross compile faucet
## [0.5.1] - 2024-01-12

### 🚀 Features

- Improves Ctrl-C exit strategy

### ⚙️ Miscellaneous Tasks

- *(CI)* Fixes workflow to publish docker images
## [0.5.0] - 2024-01-12

### 🚀 Features

- Adds `rscript-path` argument to CLI and Env vars
- Enables running arbitrary R binaries

### 🚜 Refactor

- Improves middleware with static dispatch
- Switches directory structure

### 📚 Documentation

- Documents Rscript argument on README

### ⚙️ Miscellaneous Tasks

- Adds unit testing to ip_extractor
- Adds unit tests for layers and services
- Adds unit testing for logging
- Removes test_ prefix from test functions
- Removes async-trait from websockets
- Adds unit testing for errors
- Adds unit tests for load balancing strategies
- Adds CI workflow for unit testing
- Updates tests and adds dependabot
- Adds tower manually to fix CI
- Removes codecov upload
- Adds site publishing CI
- Fixed GH Pages CI
- Fixed GH Pages CI
- Adds testing badge to README
- Adds workflow for deploying Docker images to Dockerhub
- *(CI)* Fixes workflow to publish docker images
## [0.4.2] - 2024-01-03

### 📚 Documentation

- Adds example of Plumber in an R Package

### ⚙️ Miscellaneous Tasks

- Switches repos to ixpantia
## [0.4.1] - 2023-12-15

### 🚀 Features

- Improves file detection for APIs and Shiny apps

### 🚜 Refactor

- Improves startup TcpBind
## [0.4.0] - 2023-12-14

### 🚀 Features

- Adds initial MkDocs documentation
- Adds exponential back-off and connection retries
- Creates middleware layers for logging and handling

### 🐛 Bug Fixes

- Fixes message when process exists before expected

### 🚜 Refactor

- Renames Faucet to faucet

### 📚 Documentation

- Adds how-to with docker
- Completes container tutorial in spanish
- Documents logging and options

### ⚙️ Miscellaneous Tasks

- Adds repo and base url to docs site
- Adds logo to the readme
- Changes colors on documentation
- Switches from SVG to PNG for image
## [0.3.1] - 2023-12-07

### 🚀 Features

- Adds hyper client with pooling
- Adds middleware functionality for future
- Improves IP hashing algorithm
- Changes naming convetions of load balancing strategies
- Adds different IP extractors
- Adds retrying mechanism to Workers

### 📚 Documentation

- Documents the use with a reverse proxy

### ⚙️ Miscellaneous Tasks

- Updates to tungstenite v0.21
- Updates dependencies
- Clippy fix
## [0.2.3] - 2023-12-01

### 🚀 Features

- Initial Faucet Version
- Adds support for request body payloads
- Adds support for load balancing with Hostname resolution
- Adds HTTP workers argument
- Switches to a pure Hyper server
- Fixes host
- Adds better error handling and connection queue
- Adds Shiny + Plumber load balancing support

### 🚜 Refactor

- Changes K8 args
- Changes names of K8s module

### 📚 Documentation

- Updates README with K8 mode
- Adds documentation needed
- Documents usage on README

### ⚙️ Miscellaneous Tasks

- Adds LICENSE
- Deletes old faucet files
- Merge new Faucet version
- Renames lib to faucet-server
