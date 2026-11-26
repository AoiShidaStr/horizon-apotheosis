# Dedicated Horizon server with Apotheosis compiled into its engine.
# Adapted from Horizon's packaging/server/Dockerfile. Build context: the folder written by
#   bash tools/docker/stage-context.sh <horizon source at the pinned commit> <context>
#   docker build -t horizon-apotheosis <context>
#
# The image holds none of Minecraft's files and none of the Java mods' assets. Mount the 26.3 folder
# you prepared with Horizon read-only, and put apotheosis.hmod (the same file as the clients) in
# /data/horizon-mods:
#   docker run -d --name apotheosis -p 25565:25565 --stop-timeout 180 \
#     -v apotheosis-data:/data \
#     -v /path/to/.local/horizon-client/26.3:/opt/horizon/.local/horizon-client/26.3:ro \
#     -e HORIZON_EULA=true -e HORIZON_WHITELIST=true -e HORIZON_PLAYERS=Alex -e HORIZON_OP=Alex \
#     horizon-apotheosis
FROM rust:1.99.1-slim-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config python3 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
# The server build of the engine (no sound, no controllers), with the mod: horizon-native-game.
# "fast" is Horizon's optimized, quick-to-link profile (the launcher's default for native builds).
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --profile fast -p horizon-native-game --no-default-features --bin horizon-client \
    && cargo build --profile fast -p horizon-server --bin horizon-server \
    && mkdir /out \
    && cp target/fast/horizon-client /out/horizon-engine \
    && cp target/fast/horizon-server /out/

FROM debian:bookworm-slim
RUN useradd --system --home-dir /data --shell /usr/sbin/nologin horizon \
    && mkdir -p /data /opt/horizon/.local/horizon-client \
    && chown horizon:horizon /data
COPY --from=build /out/ /opt/horizon/
ENV PATH=/opt/horizon:$PATH \
    HORIZON_NO_AUDIO=1
WORKDIR /data
VOLUME /data
EXPOSE 25565
USER horizon
# SIGTERM makes horizon-server stop the game cleanly (it saves first): give it --stop-timeout 180.
CMD ["sh", "-c", "test -f server.toml || horizon-server init --yes || exit 1; exec horizon-server run"]
