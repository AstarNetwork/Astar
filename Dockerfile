FROM debian:9.9

LABEL maintainer="takumi@staked.co.jp"

ENTRYPOINT ["/opt/target/release/plasm-node", "--dev", "--ws-external"]

WORKDIR /opt
COPY ./target-debian ./target

RUN apt-get update && \
    apt-get install -y \
    libssl-dev

EXPOSE 9944
