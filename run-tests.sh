#!/bin/sh
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

docker build --quiet --tag leda-utils-tests -f src/tests/Dockerfile.shellscripts src/
docker network create --subnet=172.18.0.0/16 ledatestnet
mkdir -p ./reports
docker run --rm -t --net ledatestnet --ip 172.18.0.2 --volume "$(pwd)/reports":/reports leda-utils-tests
RC=$?
docker network rm ledatestnet

exit $RC