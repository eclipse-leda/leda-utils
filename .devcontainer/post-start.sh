#!/bin/bash
# /********************************************************************************
# * Copyright (c) 2022 Contributors to the Eclipse Foundation
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

# This script aims to simulate (post-start of the container) as many services 
# that sdv-health looks for as possible to enable more realistic testing. 
# This requires some very specific workarounds since the init system is not even systemd. 

# setup virtual can0
sudo ip link add dev can0 type vcan || true
sudo ip link set can0 up || true

# setup openssh server service
sudo apt-get update -y
sudo apt-get install -y openssh-server
sudo systemctl enable ssh
sudo systemctl start ssh

# setup k3s and kubectl
sudo wget "https://github.com/k3s-io/k3s/releases/download/v1.22.16%2Bk3s1/k3s" -P /usr/local/bin/
sudo chmod +x /usr/local/bin/k3s
sudo cp $CODESPACE_VSCODE_FOLDER/.devcontainer/resources/kubectl /usr/local/bin/
sudo curl "https://raw.githubusercontent.com/k3s-io/k3s/master/k3s.service" -o /etc/systemd/system/k3s.service
sudo systemctl enable k3s
sudo systemctl start k3s

# add containerd service
sudo cp $CODESPACE_VSCODE_FOLDER/.devcontainer/resources/containerd.service /etc/systemd/system
sudo systemctl enable containerd
sudo systemctl start containerd

# add mosquitto service
sudo apt-get install -y mosquitto
sudo cp $CODESPACE_VSCODE_FOLDER/.devcontainer/resources/mosquitto.service /etc/systemd/system
sudo systemctl enable mosquitto
sudo systemctl start mosquitto
