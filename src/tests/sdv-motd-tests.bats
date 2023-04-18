#!/usr/bin/env bash
# /********************************************************************************
# * Copyright (c) 2023 Contributors to the Eclipse Foundation
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

setup() {
    load 'test_helper/bats-support/load.bash'
    load 'test_helper/bats-assert/load.bash'
    load 'test_helper/bats-file/load.bash'
    DIR="$( cd "$( dirname "$BATS_TEST_FILENAME" )" >/dev/null 2>&1 && pwd )"
    PATH="$DIR/../sh:$PATH"
}

@test "MOTD should show Leda" {
    run sdv-motd
    assert_output --partial 'Eclipse Leda'
}

@test "MOTD should show system info" {
    run sdv-motd
    assert_output --partial 'Hostname'
    assert_output --partial 'Network'
    assert_output --partial 'Disk Space'
    assert_output --partial 'RAM'
    assert_output --partial 'Uptime'
}

@test "MOTD should show sdv-health" {
    run sdv-motd
    assert_output --partial 'sdv-health'
}

@test "MOTD should show IP address" {
    run sdv-motd
    assert_output --partial 'Interface :'
    assert_output --partial 'eth0'
    assert_output --partial 'IP Address:'
    assert_output --partial '172.18.0.2'
}

@test "MOTD should not show 'Device does not exist'" {
    run sdv-motd
    refute_output --partial 'does not exist'
}

# Bug in parser includes a trailing whitespace in device name
@test "MOTD should not show 'Device 'eth0 ' does not exist'" {
    run sdv-motd
    refute_output --partial 'Device "eth0 " does not exist'
}

# Needs function due to pipes
get_default_route_network_device() {
    ip -o route | grep default | sed -e "s/^.*dev.//" -e "s/.proto.*//" | tr -d '[:space:]'
}
@test "Get default route network device" {
    run get_default_route_network_device
    assert_output 'eth0'
}

@test "Get output of IP ADDR SHOW" {
    run ip a s eth0
    assert_output --partial '172.18.0.2'
}

parse_output_of_ip_addr_show() {
    ip a s eth0 | grep -E -o 'inet [0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}' | cut -d' ' -f2
}
@test "Parse output of IP ADDR SHOW" {
    run parse_output_of_ip_addr_show
    assert_output '172.18.0.2'
}
