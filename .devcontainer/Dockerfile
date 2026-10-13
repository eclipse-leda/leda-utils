# /********************************************************************************
# * Copyright (c) 2022-2026 Contributors to the Eclipse Foundation
# *
# * See the NOTICE file(s) distributed with this work for additional
# * information regarding copyright ownership.
# *
# * This program and the accompanying materials are made available under the
# * terms of the Apache License 2.0 which is available at
# * https://www.apache.org/licenses/LICENSE-2.0
# *
# * SPDX-License-Identifier: Apache-2.0
# ********************************************************************************/

# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.245.2/containers/debian/.devcontainer/base.Dockerfile

# [Choice] Debian version (use bullseye on local arm64/Apple Silicon): bullseye, buster
ARG VARIANT="bookworm"
FROM mcr.microsoft.com/vscode/devcontainers/base:2-${VARIANT}

# ** [Optional] Uncomment this section to install additional packages. **
# RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
#     && apt-get -y install --no-install-recommends <your-package-list-here>

# Setup fake systemctl
RUN sudo curl https://raw.githubusercontent.com/gdraheim/docker-systemctl-replacement/master/files/docker/systemctl3.py -o /usr/local/bin/systemctl
RUN sudo chmod +x  /usr/local/bin/systemctl

# Install Dependencies
RUN apt-get update
RUN DEBIAN_FRONTEND=noninteractive apt-get -y install --no-install-recommends socat file gawk wget git diffstat unzip texinfo \
        gcc build-essential chrpath socat cpio python3 python3-pip python3-pexpect xz-utils debianutils \
        iputils-ping python3-git python3-jinja2 libegl1-mesa libsdl1.2-dev pylint xterm python3-subunit \ 
        mesa-common-dev zstd lz4 tmux mc skopeo fdisk ruby-full jq \
        libvirt-clients libvirt-daemon-system qemu-system-x86 qemu-system-arm qemu-kvm \
        squashfs-tools rauc python3-newt shellcheck cmake

RUN curl -sS https://webi.sh/shfmt | sh

RUN DEBIAN_FRONTEND=noninteractive apt-get -y install ca-certificates \
    curl \
    gnupg \
    lsb-release

# Install fake services
RUN sudo apt-get install -y openssh-server \
    && sudo apt-get install -y mosquitto mosquitto-clients

COPY resources/mosquitto.service /etc/systemd/system
COPY resources/containerd.service /etc/systemd/system

RUN mkdir -p /etc/docker
COPY resources/daemon.json /etc/docker/daemon.json


# # install kanto
RUN wget https://github.com/eclipse-kanto/kanto/releases/download/v1.0.0/kanto_1.0.0_linux_x86_64.deb -P /home/vscode \ 
   && sudo apt-get install -y /home/vscode/kanto_1.0.0_linux_x86_64.deb

# Try to enable KVM to improve performance of qemu-system-x86 on x86 hosts.
# This is also run in post-attach.sh

CMD /usr/bin/systemctl
