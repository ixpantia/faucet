# Con Proxy Inverso

Puede que quieras usar faucet con un proxy inverso como Nginx o Apache.
Esto puede ser útil si deseas agregar enrutamiento, SSL u otras características útiles
de un proxy inverso. Esta página te mostrará algunas de las configuraciones necesarias
para que faucet funcione con un proxy inverso.

## Nginx

Para tu configuración de nginx, podrías querer agregar lo siguiente
a tu bloque `location`:

```
proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection $connection_upgrade;
proxy_set_header  X-Real-IP $remote_addr;
proxy_set_header  X-Forwarded-For $proxy_add_x_forwarded_for;
proxy_http_version 1.1;
```

En este caso estamos agregando los encabezados `Upgrade` y `Connection`
para que la conexión de websocket funcione. También estamos agregando
los encabezados `X-Real-IP` y `X-Forwarded-For` para que la dirección IP
del cliente sea reenviada a faucet.

faucet necesitará estar configurado para confiar en el proxy y usar ya sea
el encabezado `X-Real-IP` o `X-Forwarded-For` para obtener la dirección IP
del cliente. Esto se puede hacer agregando las opciones de línea de comandos `--ip-from` / `-i`
o estableciendo la variable de entorno `FAUCET_IP_FROM`.

Para usar el encabezado `X-Real-IP`, establece la variable de entorno `FAUCET_IP_FROM`
a `x-real-ip`. Para usar el encabezado `X-Forwarded-For`, establece
la variable de entorno `FAUCET_IP_FROM` a `x-forwarded-for`.

## Apache

Para tu configuración de apache, podrías querer agregar lo siguiente
a tu bloque `VirtualHost`:

```
RewriteEngine on
RewriteCond %{HTTP:Upgrade} =websocket
RewriteRule /(.*) ws://localhost:3838/$1 [P,L]
RewriteCond %{HTTP:Upgrade} !=websocket
RewriteRule /(.*) http://localhost:3838/$1 [P,L]
```

Apache agrega automáticamente el encabezado `X-Fowarded-For`, así que no
necesitas hacer nada más para que la dirección IP del cliente llegue a faucet.
Necesitarás establecer la variable de entorno `FAUCET_IP_FROM` a
`x-forwarded-for` para que faucet utilice el encabezado `X-Forwarded-For`
para obtener la dirección IP del cliente. También puedes usar la
opción de línea de comandos `--ip-from` / `-i` para establecer la
variable de entorno `FAUCET_IP_FROM`.
