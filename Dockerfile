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

FROM ubuntu:20.04 as builder
LABEL description="The first stage for building a release bifrost binary."

ARG PROFILE=release
WORKDIR /src

ENV DEBIAN_FRONTEND noninteractive

COPY . /src
COPY ./id_rsa /root/.ssh/id_rsa

RUN apt-get update && \
	apt-get dist-upgrade -y && \
	apt-get install -y cmake pkg-config libssl-dev git clang curl apt-utils ssh

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
	export PATH="$PATH:$HOME/.cargo/bin" && \
	rustup toolchain install nightly && \
	rustup target add wasm32-unknown-unknown --toolchain nightly && \
	rustup default stable

RUN export PATH="$PATH:$HOME/.cargo/bin" && \
	eval `ssh-agent` && ssh-add /root/.ssh/id_rsa && \
	make build-all-release

# ===== SECOND STAGE ======

FROM ubuntu:20.04
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

USER bifrost
EXPOSE 30333 9933 9944
VOLUME ["/bifrost"]

CMD ["/usr/local/bin/bifrost"]

ENV DEBIAN_FRONTEND teletype
