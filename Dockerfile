# 1. This tells docker to use the Rust official image
FROM rust:latest
# Create blank project
RUN USER=root cargo new tiktonick
COPY Cargo.toml Cargo.lock tiktonick/
WORKDIR /tiktonick
# This is a dummy build to get the dependencies cached.
RUN cargo build --release
COPY src /tiktonick/src
# This is the actual build.
RUN cargo build --release \
    && mv target/release/tiktok-bot /bin \
    && rm -rf /tiktonick
CMD ["./bin/tiktok-bot"]