FROM ixpantia/faucet:r4.3

RUN Rscript -e "install.packages('remotes')" && \
    Rscript -e "remotes::install_github(\"sol-eng/plumbpkg\")"

COPY --chown=faucet:faucet entrypoint.R .

USER faucet
