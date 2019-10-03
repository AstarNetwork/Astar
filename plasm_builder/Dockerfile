# Use rust 1.35.0 as our base image.
FROM parity/rust:nightly

RUN apt-get update && \
    apt-get install -y \
        clang \
        llvm

WORKDIR /opt
