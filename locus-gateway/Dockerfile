FROM ubuntu:24.04 AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Binary is pre-built on the host by build-image.sh before docker build runs.
COPY locus-gateway/publish/ ./

RUN chmod +x /app/locus-gateway \
    && mkdir -p /data

ENV LOCUS_GATEWAY_HTTP_PORT=8080
ENV LOCUS_GATEWAY_GRPC_PORT=8081
ENV LOCUS_GATEWAY_ROOT_DIR_NAME=.locus-gateway
ENV HOME=/data

VOLUME ["/data"]

EXPOSE 8080
EXPOSE 8081

ENTRYPOINT ["/app/locus-gateway"]
