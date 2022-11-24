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
sudo systemctl enable ssh || true
sudo systemctl start ssh || true

# add containerd service
sudo systemctl enable containerd || true
sudo systemctl start containerd || true

# add mosquitto service
sudo systemctl enable mosquitto || true
sudo systemctl start mosquitto || true

# kanto services

sudo systemctl enable container-management.service || true
sudo systemctl enable file-backup.service || true
sudo systemctl enable file-upload.service || true
sudo systemctl enable local-digital-twins.service || true
sudo systemctl enable software-update.service || true
sudo systemctl enable suite-bootstrapping.service || true
sudo systemctl enable suite-connector.service || true
sudo systemctl enable system-metrics.service || true


sudo systemctl start container-management.service || true
sudo systemctl start file-backup.service || true
sudo systemctl start file-upload.service || true
sudo systemctl start local-digital-twins.service || true
sudo systemctl start software-update.service  || true
sudo systemctl start suite-bootstrapping.service || true
sudo systemctl start suite-connector.service || true
sudo systemctl start system-metrics.service || true