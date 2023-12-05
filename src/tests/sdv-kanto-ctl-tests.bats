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

    CONFIG_FILE="/etc/container-management/config.json"
    mkdir -p "$(dirname ${CONFIG_FILE})"
    echo "{}" > ${CONFIG_FILE}
    assert_file_exist ${CONFIG_FILE}
}


@test "SDV-KANTO-CTL should modify values in Kanto Config" {
    run sdv-kanto-ctl add-registry -h myhost -u myuser -p mypass
    assert_file_contains "${CONFIG_FILE}" '.*mypass.*'
}

@test "SDV-KANTO-CTL should modify arrays in Kanto Config" {
    run sdv-kanto-ctl add-entry containers.insecure_registries localhost:5000
    run jq '.containers.insecure_registries[0]' "${CONFIG_FILE}"
    assert_output "\"localhost:5000\""

    run sdv-kanto-ctl remove-entry containers.insecure_registries localhost:5000
    run jq '.containers.insecure_registries[0]' "${CONFIG_FILE}"
    assert_output "null"
}


