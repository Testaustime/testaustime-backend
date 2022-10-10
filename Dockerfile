FROM --platform=$BUILDPLATFORM rustlang/rust:nightly-bullseye-slim AS build

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc"

RUN apt update \
    && apt upgrade -y \
    && apt install -y git libpq-dev pkg-config libssl-dev perl make

ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
    "linux/amd64") echo "x86_64-unknown-linux-gnu" > /target.txt ;; \
    "linux/arm64") echo "aarch64-unknown-linux-gnu" > /target.txt ;; \
    *) exit 1 ;; \
esac

RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then \
    dpkg --add-architecture arm64 \
    && apt update \
    && apt install gcc-aarch64-linux-gnu libc6-dev-arm64-cross -y \
    && apt install libpq-dev:arm64 -y; \
fi

RUN rustup target add $(cat /target.txt)

RUN cargo install --target $(cat /target.txt) diesel_cli --no-default-features --features "postgres" \
    && mkdir /out \
    && cp /usr/local/cargo/bin/diesel /out

RUN cargo new --bin testaustime-rs

WORKDIR /testaustime-rs

COPY Cargo.toml Cargo.lock ./

COPY testausratelimiter/ testausratelimiter/

RUN cargo build --target $(cat /target.txt) --release && rm -rf .git src/ target/$(cat /target.txt)/release/deps/testaustime*

COPY src/ src/

RUN cargo build --target $(cat /target.txt) --release && mv target/$(cat /target.txt)/release/testaustime-rs /out



FROM --platform=$TARGETPLATFORM debian:bullseye-slim AS runner

RUN apt update \
    && apt upgrade -y \
    && apt install --no-install-recommends postgresql-client ca-certificates -y \
    && rm -rf /var/lib/apt/lists/*

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/none" \
    --shell "/sbin/nologin" \
    --no-create-home \
    testaustime

WORKDIR /app

COPY --from=build /out/diesel ./
COPY --from=build /out/testaustime-rs ./
COPY migrations /app/migrations
COPY entrypoint.sh ./

RUN chown -R testaustime:testaustime /app

USER testaustime

CMD [ "sh", "entrypoint.sh" ]
