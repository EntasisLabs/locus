FROM ubuntu:24.04 AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Binary is pre-built on the host by build-image.sh before docker build runs.
COPY src/sttp/locus-mcp/publish/ ./

RUN chmod +x /app/locus-mcp \
    && mkdir -p /data

# Keep all runtime state under the mounted data volume by default.
ENV HOME=/data

VOLUME ["/data"]

ENTRYPOINT ["/app/locus-mcp"]
