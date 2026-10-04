# syntax=docker/dockerfile:1
#
# bichinho: um binário estático (musl) numa imagem alpine mínima.
# Sem Mesa, sem /dev/dri, sem áudio: os pixels vão por SHM e quem compõe é
# a GPU do Hyprland (decisões 0002 e 0004).
#
# Bases fixadas por digest (decisão 0014). Para atualizar, troque a tag e
# pegue o digest novo com:
#   docker buildx imagetools inspect rust:<versão>-alpine<versão>
#   docker buildx imagetools inspect alpine:<versão>

FROM rust:1.98.1-alpine3.24@sha256:7cc1c22d77d9432f7fe012a70e6d3e555af54c2a6832700ed7d553f1769ae89f AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
# Sem rust-toolchain.toml aqui: o compilador é o da imagem (nada é baixado).
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY crates ./crates
COPY xtask ./xtask
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p bichinho \
 && install -Dm0755 target/release/bichinho /out/bichinho

FROM alpine:3.24.2@sha256:294b683cb724975bec92580e1e685676bd4b50bda910ddb8c51d4cabeaec77e6 AS runtime
ARG APP_UID=1000
ARG APP_GID=1000
RUN set -eux; \
    addgroup -g "$APP_GID" pet; \
    adduser -D -H -u "$APP_UID" -G pet -h /state -s /sbin/nologin pet; \
    install -d -o "$APP_UID" -g "$APP_GID" -m 0750 /state
COPY --from=build /out/bichinho /usr/local/bin/bichinho
COPY assets/ /opt/bichinho/assets/
COPY skins/ /opt/bichinho/skins/
# Arte que não pode ser redistribuída: fora do git, mas entra na imagem
# LOCAL (que nunca vai a registry). Decisão 0011.
COPY skins-locais/ /opt/bichinho/skins-locais/
ENV PET_HOST_RUNTIME=/host/run/user \
    PET_ESCUTA=0.0.0.0:27380 \
    PET_ASSETS=/opt/bichinho/assets \
    PET_SKINS=/opt/bichinho/skins:/opt/bichinho/skins-locais \
    PET_ESTADO=/state \
    PET_CONFIG=/etc/bichinho \
    HOME=/tmp \
    XDG_RUNTIME_DIR=/tmp/xdg
USER ${APP_UID}:${APP_GID}
EXPOSE 27380
ENTRYPOINT ["/usr/local/bin/bichinho"]
CMD ["rodar"]
