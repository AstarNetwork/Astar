FROM phusion/baseimage:0.10.2

LABEL maintainer="takumi@stake.co.jp"

ENTRYPOINT ["/opt/plasm-node"]

WORKDIR /opt
COPY ./target-debian/release/plasm-node /opt/plasm-node

# Shrinking
RUN rm -rf /usr/lib/python* && \
	rm -rf /usr/bin /usr/sbin /usr/share/man

VOLUME ["/data"]

EXPOSE 9944
