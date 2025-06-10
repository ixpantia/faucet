# With Reverse Proxy

You may want to use faucet with a reverse proxy like Nginx or Apache.
This might be useful if you want to add routing, SSL, or other useful features
of a reverse proxy. This page will show you some of the necessary configuration
to get faucet working with a reverse proxy.

## Nginx

For your nginx configuration, you might want to add the following
to your `location` block:

```
proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection $connection_upgrade;
proxy_set_header  X-Real-IP $remote_addr;
proxy_set_header  X-Forwarded-For $proxy_add_x_forwarded_for;
proxy_http_version 1.1;
```

In this case we are adding the `Upgrade` and `Connection` headers
so that the websocket connection will work. We are also adding
the `X-Real-IP` and `X-Forwarded-For` headers so that the IP address
of the client will be forwarded to faucet.

faucet will need to be configured to trust the proxy and use either
the `X-Real-IP` or `X-Forwarded-For` header to get the IP address
of the client. This can be done by adding the `--ip-from` / `-i`
command line options or by setting the `FAUCET_IP_FROM` environment
variable.

To use the `X-Real-IP` header, set the `FAUCET_IP_FROM` environment
variable to `x-real-ip`. To use the `X-Forwarded-For` header, set
the `FAUCET_IP_FROM` environment variable to `x-forwarded-for`.

## Apache

For your apache configuration, you might want to add the following
to your `VirtualHost` block:

```
RewriteEngine on
RewriteCond %{HTTP:Upgrade} =websocket
RewriteRule /(.*) ws://localhost:3838/$1 [P,L]
RewriteCond %{HTTP:Upgrade} !=websocket
RewriteRule /(.*) http://localhost:3838/$1 [P,L]
```

Apache automatically adds the `X-Fowarded-For` header, so you don't
need to do anything else to get the client IP address to faucet.
You will need to set the `FAUCET_IP_FROM` environment variable to
`x-forwarded-for` so that faucet will use the `X-Forwarded-For`
header to get the IP address of the client. You can also use the
`--ip-from` / `-i` command line option to set the `FAUCET_IP_FROM`
environment variable.

Similar to the Nginx setup, when Apache proxies WebSocket requests (as shown in the `RewriteRule` with the `ws://` scheme), Faucet will actively manage this connection. It receives the upgrade request, performs the necessary WebSocket handshake with your backend R application, and then proxies the WebSocket data.
