# Copyright 2019-2021 Liebi Technologies.
# This file is part of Bifrost.

# Bifrost is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

# Bifrost is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.

# You should have received a copy of the GNU General Public License
# along with Bifrost.  If not, see <http:#www.gnu.org/licenses/>.

FROM ubuntu:latest as builder
LABEL description="The first stage for building a release bifrost binary."

ARG PROFILE=release
WORKDIR /src

ENV DEBIAN_FRONTEND noninteractive

COPY . /src

RUN apt-get update && \
	apt-get dist-upgrade -y && \
	apt-get install -y cmake pkg-config libssl-dev git clang curl apt-utils

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
	export PATH="$PATH:$HOME/.cargo/bin" && \
	rustup toolchain install nightly && \
	rustup target add wasm32-unknown-unknown --toolchain nightly && \
	rustup default nightly && \
	rustup default stable && \
	cargo build "--$PROFILE"

# ===== SECOND STAGE ======

FROM ubuntu:latest
LABEL description="The second stage for configuring the image."
ARG PROFILE=release
WORKDIR /bifrost

RUN apt-get update && \
	apt-get dist-upgrade -y && \
	apt install -y openssl libssl-dev

RUN rm -rf /usr/share/* && \
	useradd -m -u 1000 -U -s /bin/sh -d /bifrost bifrost && \
	mkdir -p /bifrost/.local && \
	chown -R bifrost:bifrost /bifrost/.local

COPY --from=builder /src/target/$PROFILE/bifrost /usr/local/bin

# checks
RUN ldd /usr/local/bin/bifrost && \
	/usr/local/bin/bifrost --version

# Shrinking
RUN rm -rf /usr/lib/python* && \
	rm -rf /usr/bin /usr/sbin /usr/share/man && \
	rm -rf /src

USER bifrost
EXPOSE 30333 9933 9944
VOLUME ["/bifrost"]

CMD ["/usr/local/bin/bifrost"]

ENV DEBIAN_FRONTEND teletype
