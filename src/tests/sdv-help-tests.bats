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

@test "HELP should show commands" {
    run sdv-help
    assert_output --partial 'sdv-health'
    assert_output --partial 'kantui'
    assert_output --partial 'kanto-auto-deployer'
    assert_output --partial 'sdv-provision'
    assert_output --partial 'mosquitto'
    assert_output --partial 'loadkeys'
}
