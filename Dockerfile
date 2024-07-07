###################################################################################
## Builder
###################################################################################
FROM rust:slim-bullseye AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev pkg-config libssl-dev upx make
RUN update-ca-certificates

# Create appuser
ENV USER=rust
ENV UID=1001

RUN adduser \
  --disabled-password \
  --gecos "" \
  --home "/" \
  --shell "/sbin/nologin" \
  --no-create-home \
  --uid "${UID}" \
  "${USER}"


WORKDIR /workdir

COPY ./ .

RUN cargo build --target x86_64-unknown-linux-musl --release
RUN upx --best --lzma target/x86_64-unknown-linux-musl/release/az-group-manager

###################################################################################
## Final image
###################################################################################
FROM scratch

WORKDIR /

# Copy from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group
COPY --from=builder /etc/ssl/certs /etc/ssl/certs
COPY --from=builder /workdir/target/x86_64-unknown-linux-musl/release/az-group-manager/ /

# Use an unprivileged user.
USER 1001:1001

ENTRYPOINT ["./az-group-manager"]
