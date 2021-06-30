FROM debian:buster-slim

# metadata
ARG VCS_REF
ARG BUILD_DATE

LABEL maintainer="takumi@stake.co.jp"
# show backtraces
ENV RUST_BACKTRACE 1

# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get upgrade -y && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y \
	libssl1.1 \
	ca-certificates \
	build-essential \
	curl && \
	# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
	# add user
	useradd -m -u 1000 -U -s /bin/sh -d /plasm plasm

# add plasm-node binary to docker image
WORKDIR /opt
COPY ./target-debian/release/plasm /opt/plasm

USER plasm

RUN gcc --version
# check if executable works in this container
RUN /opt/plasm --version

VOLUME ["/data"]

EXPOSE 30333 9933 9944

ENTRYPOINT ["/opt/plasm"]
