FROM docker.io/library/ubuntu:jammy AS builder
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


FROM docker.io/library/ubuntu:jammy AS build-r
ARG R_VERSION

ENV R_VERSION=${R_VERSION}
ENV R_HOME="/usr/local/lib/R"

COPY scripts/install_R_source.sh /rocker_scripts/install_R_source.sh
RUN /rocker_scripts/install_R_source.sh

ENV CRAN="https://p3m.dev/cran/__linux__/jammy/latest"
ENV LANG=en_US.UTF-8

COPY scripts/bin/ /rocker_scripts/bin/
COPY scripts/setup_R.sh /rocker_scripts/setup_R.sh
RUN /rocker_scripts/setup_R.sh

FROM build-r AS faucet

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
