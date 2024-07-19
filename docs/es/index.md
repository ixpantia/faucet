# faucet ![logo](figures/faucet.png){ align=right height=139 width=120 }

<!-- insignias: inicio -->
[![Crates.io](https://img.shields.io/crates/v/faucet-server.svg)](https://crates.io/crates/faucet-server)
<!-- insignias: fin -->

Despliegue Rápido, Asincrónico y Concurrente de Aplicaciones R

---

## Introducción

Bienvenido a faucet, tu solución de alto rendimiento para desplegar APIs de Plumber y Aplicaciones Shiny con velocidad y eficiencia. faucet es un servidor basado en Rust que ofrece equilibrio de carga Round Robin e IP Hash, garantizando una escalabilidad y distribución fluidas de tus aplicaciones R. Ya seas un científico de datos, desarrollador o entusiasta de DevOps, faucet simplifica el despliegue, facilitando la gestión de réplicas y el equilibrio de cargas de manera efectiva.

## Características

- **Alto Rendimiento:** faucet aprovecha la velocidad de Rust para una ejecución suave y eficiente de las aplicaciones R.
- **Equilibrio de Carga:** Elige el equilibrio de carga Round Robin o IP Hash para una utilización óptima de los recursos.
- **Réplicas:** Escala las APIs de Plumber y las Aplicaciones Shiny sin esfuerzo con múltiples réplicas.
- **Despliegue Simplificado:** faucet simplifica el proceso de despliegue para una configuración rápida.
- **Asincrónico y Concurrente:** Utiliza el procesamiento asíncrono y concurrente para una eficiencia de recursos y una manipulación de solicitudes receptiva.

## Instalación

Para opciones de instalación, consulta [Instalación](./install.md).

## Modos de Uso

### Single Server: 

El modo Single Server es adecuado cuando tienes una sola aplicación que deseas desplegar. Este modo permite iniciar y gestionar una única "instancia" de una aplicación Plumber o Shiny.

### Router: 

El modo Router es ideal cuando tienes varias aplicaciones (Shiny, Quarto, Plumber) por desplegar y deseas que cada aplicación este en un mismo puerto pero en diferentes rutas. El Router se encarga de gestionar las rutas y dirigir las solicitudes a la aplicación correspondiente.

Para instrucciones detalladas de los modos de uso, consulta [Cómo Empezar](./getting_started.md).

## Con Docker / en Contenedores

faucet también está disponible como una imagen de Docker, para instrucciones
detalladas de uso con Docker, consulta [faucet en
Contenedores](./in_containers.md).
