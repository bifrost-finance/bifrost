# Copyright (C) Liebi Technologies PTE. LTD.
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
FROM rust:buster as builder

RUN apt-get update && apt-get install time cmake clang libclang-dev llvm protobuf-compiler -y
RUN rustup toolchain install 1.77.0
RUN rustup target add wasm32-unknown-unknown --toolchain 1.77.0

WORKDIR /app
COPY . /app

RUN export PATH="$PATH:$HOME/.cargo/bin" && \
	make production-release

# ===== SECOND STAGE ======

FROM ubuntu:20.04

RUN rm -rf /usr/share  && \
  rm -rf /usr/lib/python* && \
  useradd -m -u 1000 -U -s /bin/sh -d /bifrost bifrost && \
  chown -R bifrost:bifrost /bifrost && \
  mkdir -p /bifrost/.local/share && \
  mkdir /data && \
  chown -R bifrost:bifrost /data && \
  ln -s /data /bifrost/.local/share/bifrost && \
  mkdir /spec && \
  chown -R bifrost:bifrost /spec && \
  ln -s /spec /bifrost/.local/share/spec

USER bifrost
COPY --from=builder /app/target/production/bifrost /usr/local/bin
COPY ./node/service/res/bifrost-kusama.json /spec/bifrost.json
COPY ./node/service/res/bifrost-kusama.json /spec
COPY ./node/service/res/bifrost-polkadot.json /spec

# checks
RUN ldd /usr/local/bin/bifrost && \
  /usr/local/bin/bifrost --version

USER bifrost
EXPOSE 30333 9933 9944

VOLUME ["/data"]
VOLUME ["/spec"]

ENTRYPOINT ["/usr/local/bin/bifrost"]
