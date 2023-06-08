FROM rust:1.70.0
LABEL authors="liangw"

WORKDIR /app
COPY . /app
RUN cargo build --release

ENTRYPOINT ["cargo", "run", "--release"]