#!/bin/bash
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

# This script aims to simulate (post-start of the container) as many services 
# that sdv-health looks for as possible to enable more realistic testing. 
# This requires some very specific workarounds since the init system is not even systemd. 

# setup virtual can0
echo "Configuring CAN interface..."
if ! sudo ip link add dev can0 type vcan 2>&1; then
	echo "WARNING: Failed to create virtual CAN interface 'can0' (vcan module missing or not supported)."
	echo "         If you are running on WSL2, please install a custom WSL2 kernel with VCAN support."
else
	sudo ip link set can0 up || true
fi

# setup openssh server service
echo "Starting SSH service..."
sudo systemctl enable ssh >/dev/null 2>&1 || true
sudo systemctl start ssh >/dev/null 2>&1 || true

# add containerd service
echo "Starting containerd..."
sudo systemctl enable containerd >/dev/null 2>&1 || true
sudo systemctl start containerd >/dev/null 2>&1 || true

# add mosquitto service
echo "Starting mosquitto..."
sudo systemctl enable mosquitto >/dev/null 2>&1 || true
sudo systemctl start mosquitto >/dev/null 2>&1 || true

# kanto services
echo "Starting Eclipse Kanto services..."
sudo systemctl enable container-management.service >/dev/null 2>&1 || true
sudo systemctl enable file-backup.service >/dev/null 2>&1 || true
sudo systemctl enable file-upload.service >/dev/null 2>&1 || true
sudo systemctl enable kanto-update-manager.service >/dev/null 2>&1 || true
sudo systemctl enable local-digital-twins.service >/dev/null 2>&1 || true
sudo systemctl enable software-update.service >/dev/null 2>&1 || true
sudo systemctl enable suite-connector.service >/dev/null 2>&1 || true
sudo systemctl enable system-metrics.service >/dev/null 2>&1 || true

echo "... container-management"
sudo systemctl start container-management.service >/dev/null 2>&1 || true
echo "... file-backup"
sudo systemctl start file-backup.service >/dev/null 2>&1 || true
echo "... file-upload"
sudo systemctl start file-upload.service >/dev/null 2>&1 || true
echo "... kanto-update-manager"
sudo systemctl start kanto-update-manager.service >/dev/null 2>&1 || true
echo "... local-digital-twins"
sudo systemctl start local-digital-twins.service >/dev/null 2>&1 || true
echo "... software-update"
sudo systemctl start software-update.service >/dev/null 2>&1 || true
echo "... suite-connector"
sudo systemctl start suite-connector.service >/dev/null 2>&1 || true
echo "... system-metrics"
sudo systemctl start system-metrics.service >/dev/null 2>&1 || true

echo "Done, all services started."
