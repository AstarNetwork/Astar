FROM debian:buster-slim

# metadata
ARG VCS_REF
ARG BUILD_DATE
ARG IMAGE_NAME

LABEL io.plasm.image.authors="takumi@stake.co.jp" \
	io.plasm.image.vendor="Stake Technologies" \
	io.plasm.image.title="${IMAGE_NAME}" \
	io.plasm.image.description="plasm: a platform for web3" \
	io.plasm.image.source="https://github.com/PlasmNetwork/Plasm/blob/${VCS_REF}/third-parity/docker/Dockerfile" \
	io.plasm.image.revision="${VCS_REF}" \
	io.plasm.image.created="${BUILD_DATE}" \
	io.plasm.image.documentation="https://github.com/PlasmNetwork/plasm/"

# show backtraces
ENV RUST_BACKTRACE 1

# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y \
	libssl1.1 \
	ca-certificates \
	curl && \
	# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
	# add user and link ~/.local/share/plasm to /data
	useradd -m -u 1000 -U -s /bin/sh -d /plasm plasm && \
	mkdir -p /data /plasm/.local/share && \
	chown -R plasm:plasm /data && \
	ln -s /data /plasm/.local/share/plasm

# add plasm binary to docker image
COPY ./target-debian/release/plasm /usr/local/bin

USER plasm

# check if executable works in this container
RUN /usr/local/bin/plasm --version

EXPOSE 30333 9933 9944
VOLUME ["/plasm"]

ENTRYPOINT ["/usr/local/bin/plasm"]
