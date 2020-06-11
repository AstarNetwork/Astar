# Use parity rust-builder as our base image.
FROM paritytech/ci-linux:production

RUN apt-get update && \
    apt-get install -y \
        clang \
        llvm

WORKDIR /opt
