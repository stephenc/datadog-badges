FROM ekidd/rust-musl-builder:1.41.0 AS build

# Build an empty project with our dependencies so that we can cache the compiled dependencies
RUN USER=root cargo init .
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

# Add our source code.
COPY src/ ./src/

# Build for real
RUN cargo install --target x86_64-unknown-linux-musl --path .

# Now for the runtime image
FROM scratch

COPY --from=build /etc/ssl /etc/ssl
COPY --from=build /home/rust/.cargo/bin/datadog-badges /datadog-badges

USER 1000

ENTRYPOINT ["/datadog-badges"]
EXPOSE 8080
