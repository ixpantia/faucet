# faucet ![logo](figures/faucet.png){ align=right height=139 width=120 }

<!-- badges: start -->
[![Crates.io](https.img.shields.io/crates/v/faucet-server.svg)](https://crates.io/crates/faucet-server)
<!-- badges: end -->

Despliegue Rápido, Asíncrono y Concurrente de Aplicaciones R y Python

---

## Resumen

Bienvenido a faucet, tu solución de alto rendimiento para desplegar APIs de Plumber, Aplicaciones Shiny y aplicaciones FastAPI con velocidad y eficiencia. Faucet es un servidor basado en Rust que ofrece balanceo de carga Round Robin, IP Hash y Cookie Hash, asegurando un escalado y distribución fluidos de tus aplicaciones R y Python. También permite ejecutar scripts arbitrarios de R y Python. Ya seas un científico de datos, desarrollador o entusiasta de DevOps, faucet simplifica el despliegue, facilitando la gestión de réplicas y el balanceo de cargas de manera efectiva.

## Características

- **Alto Rendimiento:** faucet aprovecha la velocidad de Rust para una ejecución fluida y eficiente de aplicaciones R y Python.
- **Soporte Políglota:** Despliega nativamente aplicaciones escritas en R (Plumber, Shiny) y Python (FastAPI), o ejecuta scripts arbitrarios de `Rscript` y Python (`uv`).
- **Balanceo de Carga:** Elige entre balanceo de carga Round Robin, IP Hash o Cookie Hash para una utilización óptima de los recursos.
- **Réplicas:** Escala APIs de Plumber, Aplicaciones Shiny y aplicaciones FastAPI sin esfuerzo con múltiples réplicas.
- **Despliegue Simplificado:** faucet agiliza el proceso de despliegue para una configuración rápida.
- **Asíncrono y Concurrente:** Utiliza procesamiento asíncrono y concurrente para una mayor eficiencia de recursos y un manejo receptivo de las solicitudes.
- **Trazado Estructurado de Eventos:** Obtén información detallada sobre tus aplicaciones Shiny con registros detallados y legibles por máquina almacenados directamente en tu base de datos.


## Instalación

Para opciones de instalación, consulta [Instalación](./install.md).

## Uso

Para instrucciones de uso detalladas, consulta [Primeros Pasos](./getting_started.md).

## Con Docker

faucet también está disponible como imagen de Docker. Para instrucciones de uso detalladas con
Docker, consulta [faucet en Contenedores](./in_containers.md).