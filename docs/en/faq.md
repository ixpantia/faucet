# Frequently Asked Questions

### faucet is not load balancing my Shiny App on Google Cloud Run.

Google Cloud Run has a proxy between the requests sent and the actual
underlying services. Therefore we need to tell faucet who is connecting
and how to read the end-user's IP address.

We can fix this by setting the `FAUCET_IP_FROM` environment variable or
`--ip-from` CLI argument to `x-forwarded-for`.
