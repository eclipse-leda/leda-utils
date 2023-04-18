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

@test "HEALTH should show Leda" {
    run sdv-health
    assert_output --partial 'Leda'
    assert_output --partial '0.0.0'
}

@test "HEALTH should show can0" {
    run sdv-health
    assert_output --partial 'can0'
}

@test "HEALTH should show SDV containers" {
    run sdv-health
    assert_output --partial 'databroker'
    assert_output --partial 'Kanto'
}
