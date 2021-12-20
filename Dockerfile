# 1. This tells docker to use the Rust official image
FROM rust:latest
COPY ./ ./
RUN cargo build --release
CMD ["./target/release/tiktok-bot"]