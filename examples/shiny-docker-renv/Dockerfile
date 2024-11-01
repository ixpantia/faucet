FROM ixpantia/faucet:r4.4

# Install extra system dependencies
RUN apt-get update && \
    apt-get install -y libssl-dev libcurl4-openssl-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy Files to bootstrap renv
COPY renv.lock renv.lock
COPY .Rprofile .Rprofile
COPY renv/ renv/

# Restore Renv
RUN Rscript -e "renv::restore()"

COPY global.R global.R
COPY ui.R ui.R
COPY server.R server.R
