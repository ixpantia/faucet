# Frequently Asked Questions

### faucet is not load balancing my Shiny App on Google Cloud Run.

Google Cloud Run has a proxy between the requests sent and the actual
underlying services. Therefore we need to tell faucet who is connecting
and how to read the end-user's IP address.

We can fix this by setting the `FAUCET_IP_FROM` environment variable or
`--ip-from` CLI argument to `x-forwarded-for`.

### I'm getting "address already in use" errors with my workers.

If you see errors like `createTcpServer: address already in use` or `Failed to create server`, this typically means that your application code has hardcoded port settings that conflict with faucet's port management.

Faucet automatically assigns unique ports to each worker, but your application code might be overriding these with explicit port declarations.

**Common causes and solutions:**

- **Shiny apps:** Check for `options(shiny.port = ...)` calls in your code and remove them. Also avoid hardcoded ports in `shiny::runApp(port = ...)` calls.
- **Plumber APIs:** Remove explicit port settings in `plumber::pr_run(port = ...)` calls.
- **Other services:** Ensure no hardcoded ports are set in configuration files or startup scripts.

Let faucet manage the port assignments automatically for proper load balancing to work.
