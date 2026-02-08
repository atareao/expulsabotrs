###############################################################################
## Builder
###############################################################################
FROM rust:alpine3.23 AS builder

LABEL maintainer="Lorenzo Carbonell <a.k.a. atareao> lorenzo.carbonell.cerezo@gmail.com"

RUN apk add --update --no-cache \
    autoconf \
    gcc \
    gdb \
    make \
    musl-dev

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src src

RUN cargo build --release && \
    cp /app/target/release/expulsabot /app/expulsabot

###############################################################################
## Final image
###############################################################################
FROM alpine:3.23

ENV USER=app
ENV UID=10001

RUN apk add --update --no-cache \
    ca-certificates \
    curl \
    openssl \
    tzdata~=2026 && \
    rm -rf /var/cache/apk && \
    rm -rf /var/lib/app/lists*

# Configure timezone to Europe/Madrid
RUN ln -sf /usr/share/zoneinfo/Europe/Madrid /etc/localtime && \
    echo "Europe/Madrid" > /etc/timezone

# Copy our build
COPY --from=builder /app/expulsabot /app/

# Set the work dir
WORKDIR /app
#USER app

CMD ["/app/expulsabot"]