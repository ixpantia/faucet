ARG R_VERSION

FROM rocker/r-ver:${R_VERSION} AS builder
RUN apt-get update && \
    apt-get install -y curl build-essential && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    curl https://sh.rustup.rs -sSf | bash -s -- -y && \
    mkdir faucet_src
COPY src faucet_src/src
COPY Cargo.toml faucet_src/Cargo.toml
COPY Cargo.lock faucet_src/Cargo.lock
RUN /root/.cargo/bin/cargo install --path faucet_src

FROM rocker/r-ver:${R_VERSION} AS faucet

LABEL org.opencontainers.image.licenses="GPL-2.0-or-later" \
      org.opencontainers.image.source="https://github.com/ixpantia/faucet" \
      org.opencontainers.image.vendor="faucet" \
      org.opencontainers.image.authors="Andrés F. Quintero <afquinteromoreano@gmail.com>"

RUN apt-get update && \
    apt-get install -y libsodium-dev libz-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /root/.cargo/bin/faucet /usr/local/bin/faucet
RUN useradd faucet && \
    mkdir /srv/faucet && \
    chown faucet /srv/faucet && \
    usermod -d /srv/faucet faucet && \
    chown -R faucet /usr/local/lib/R/site-library

WORKDIR /srv/faucet
EXPOSE 3838

ENV FAUCET_HOST=0.0.0.0:3838
ENV FAUCET_DIR=/srv/faucet
ENV FAUCET_IP_FROM=client

ENTRYPOINT ["/usr/local/bin/faucet"]
CMD ["start"]
