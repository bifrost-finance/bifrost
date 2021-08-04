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

# syntax=docker/dockerfile:1
FROM ubuntu:20.04 as builder

ENV DEBIAN_FRONTEND noninteractive

ENV PATH=$PATH:$HOME/.cargo/bin

RUN apt-get update && \
	apt-get dist-upgrade -y && \
	apt-get install -y cmake pkg-config libssl-dev git clang libclang-dev curl apt-utils openssh-client

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
	export PATH="$PATH:$HOME/.cargo/bin" && \
	rustup default nightly && \
	rustup target add wasm32-unknown-unknown --toolchain nightly

WORKDIR /app
COPY . /app
RUN mkdir -p -m 0600 ~/.ssh && ssh-keyscan github.com >> ~/.ssh/known_hosts

RUN --mount=type=ssh export PATH="$PATH:$HOME/.cargo/bin" && \
	make build-all-release

# ===== SECOND STAGE ======

FROM ubuntu:20.04
WORKDIR /bifrost

RUN rm -rf /usr/share/*  && \
  rm -rf /usr/lib/python* && \
  useradd -m -u 1000 -U -s /bin/sh -d /bifrost bifrost && \
  mkdir -p /bifrost/.local/data && \
  chown -R bifrost:bifrost /bifrost && \
  ln -s /bifrost/.local/data /data

COPY --from=builder /app/target/release/bifrost /usr/local/bin
COPY ./node/service/res/asgard.json /bifrost
COPY ./node/service/res/bifrost.json /bifrost

# checks
RUN ldd /usr/local/bin/bifrost && \
  /usr/local/bin/bifrost --version


USER bifrost
EXPOSE 30333 9933 9944

VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/bifrost"]