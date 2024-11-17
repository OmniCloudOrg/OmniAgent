# syntax=docker/dockerfile:1
ARG RUST_VERSION=1.82.0
ARG APP_NAME=OmniAgent

FROM rust:${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app
RUN apk add --no-cache clang lld musl-dev git
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
#    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --locked --release && \
    cp ./target/release/$APP_NAME /bin/server

FROM alpine:3.18 AS final
RUN apk add --no-cache docker-cli socat

# Copy the executable from the "build" stage
COPY --from=build /bin/server /bin/

# Expose the ports that the application listens on
EXPOSE 8080
EXPOSE 2375

# Remove the USER directive to run as root
# USER appuser  <- Remove this line

CMD ["/bin/sh", "-c", "socat TCP-LISTEN:2375,reuseaddr,fork UNIX-CONNECT:/var/run/docker.sock & sleep 2 && /bin/server"]
