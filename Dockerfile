FROM rust:latest

RUN dpkg --add-architecture amd64 && \
    apt-get update && apt-get install -y \
    build-essential \
    gcc \
    cmake \
    musl-tools \
    pkg-config \
    libssl-dev:amd64 \
    zlib1g-dev:amd64 \
    ca-certificates \
    libcurl4-openssl-dev \
    gcc-x86-64-linux-gnu \
    g++-x86-64-linux-gnu \
    libclang-dev \
    clang \
    curl && \
    rustup target add x86_64-unknown-linux-gnu

ENV CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc
ENV CXX_x86_64_unknown_linux_gnu=x86_64-linux-gnu-g++
ENV RUSTFLAGS="-C linker=x86_64-linux-gnu-gcc"
ENV OPENSSL_INCLUDE_DIR=/usr/include/x86_64-linux-gnu
ENV OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu

WORKDIR /usr/src/myapp
COPY . .

RUN cargo build --release --target x86_64-unknown-linux-gnu
