FROM ubuntu:20.04 as builder

# metadata
ARG VCS_REF
ARG BUILD_DATE

LABEL io.parity.image.authors="devops@stake.co.jp" \
	io.parity.image.vendor="Stake Technologies" \
	io.parity.image.title="PlasmNetwork/Astar" \
	io.parity.image.description="Astar: " \
	io.parity.image.source="https://github.com/PlasmNetwork/Astar/blob/${VCS_REF}/third-party/docker/Dockerfile" \
	io.parity.image.revision="${VCS_REF}" \
	io.parity.image.created="${BUILD_DATE}" \
	io.parity.image.documentation="https://docs.astar.network"

# show backtraces
ENV RUST_BACKTRACE 1

# add user
RUN useradd -m -u 1000 -U -s /bin/sh -d /astar astar && \
   	mkdir /data && \
    	chown -R astar:astar /data && \
    	rm -rf /usr/bin /usr/sbin

ARG PROFILE=release
# add binary to docker image
COPY ./astar-collator /usr/local/bin

USER astar 

# check if executable works in this container
RUN ["astar-collator", "--version"]

EXPOSE 30333 30334 9933 9944
VOLUME ["/data"]

CMD ["/usr/local/bin/astar-collator","-d","/data"]
