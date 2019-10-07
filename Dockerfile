FROM phusion/baseimage:0.10.2

LABEL maintainer="takumi@staked.co.jp"

ENTRYPOINT ["/opt/target/release/plasm-node", "--ws-external"]

WORKDIR /opt
COPY ./target-debian ./target

# Shrinking
RUN rm -rf /usr/lib/python* && \
	rm -rf /usr/bin /usr/sbin /usr/share/man

EXPOSE 9944
