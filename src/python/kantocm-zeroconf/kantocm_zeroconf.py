#!/usr/bin/env python3
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
# pylint: disable=missing-function-docstring, missing-class-docstring, missing-module-docstring
# pylint: disable=import-error

import argparse
import asyncio
import logging
import signal
import socket

from dataclasses import dataclass

import grpc

from zeroconf import IPVersion, ServiceInfo, Zeroconf, NonUniqueNameException

from kanto_cm.api.services.containers import containers_pb2
from kanto_cm.api.services.containers import containers_pb2_grpc


log = logging.getLogger("kantocm-zeroconf")


@dataclass
class ContainerStatus:
    c_id: str
    name: str
    ports: list[tuple]
    running: bool = False


class KantocmZeroconf:
    def __init__(self, socket_path: str, interval: int):
        self.interval = interval
        self.containers = {}
        self.stop_event = asyncio.Event()
        self.kanto = KantoClient(socket_path)
        self.zeroconf = ZeroconfClient()
        signal.signal(signal.SIGINT, self.stop)
        signal.signal(signal.SIGTERM, self.stop)

    async def run(self):
        log.info("Starting up")
        await self.update_containers()
        await self.unpublish_all_containers()

    def stop(self, *args):
        # pylint: disable=unused-argument
        log.info("Shutting down")
        self.stop_event.set()
        self.kanto.close_connection()

    async def update_containers(self):
        while not self.stop_event.is_set():
            log.debug("Updating container status")
            container_list = self.kanto.get_container_status()
            current_containers = {}

            for container in container_list:
                try:
                    ports = sorted([(mapping.host_port, mapping.protocol)
                             for mapping in container.host_config.port_mappings])
                except TypeError:
                    ports = []

                status = ContainerStatus(
                    c_id=container.id,
                    name=container.name,
                    running=container.state.running,
                    ports=ports
                )

                # container added?
                if status.c_id not in self.containers:
                    await self.update_service(status, True)

                current_containers[container.id] = status

            # containers removed?
            for c_id in self.containers.keys() - current_containers.keys():
                await self.update_service(self.containers[c_id], False)

            # containers changed?
            for c_id in set(current_containers.keys()).intersection(self.containers.keys()):
                running_now = current_containers[c_id].running
                if running_now:
                    # container started?
                    if not self.containers[c_id].running:
                        await self.update_service(self.containers[c_id], True)
                    else:
                        # ports changed?
                        if current_containers[c_id].ports != self.containers[c_id].ports:
                            if current_containers[c_id].ports:
                                await self.update_service(current_containers[c_id], True)
                            else:
                                await self.update_service(self.containers[c_id], False)
                else:
                    # container stopped?
                    if self.containers[c_id].running:
                        await self.update_service(self.containers[c_id], False)

            self.containers = current_containers

            log.debug("Sleeping for %s second(s)", self.interval)
            await asyncio.sleep(self.interval)

    async def update_service(self, data: ContainerStatus, publish: bool):
        if publish:
            if data.ports:
                await self.zeroconf.publish_service(data)
        else:
            await self.zeroconf.unpublish_service(data.c_id)

    async def unpublish_all_containers(self):
        for c_id in self.containers:
            await self.zeroconf.unpublish_service(c_id)

class KantoClient:
    def __init__(self, socket_path: str):
        self.socket_path = "unix:///" + socket_path
        self.channel = grpc.insecure_channel(self.socket_path)
        self.stub = containers_pb2_grpc.ContainersStub(self.channel)

    def get_container_status(self) -> list:
        log.debug("Fetching container status")
        response = self.stub.List(containers_pb2.ListContainersRequest())
        log.debug("%s containers found", len(response.containers))
        return response.containers

    def close_connection(self):
        self.channel.close()


class ZeroconfClient:
    def __init__(self):
        self.services = {}
        self.ip_version = IPVersion.All
        self.server = f"{socket.gethostname()}.local."
        self.zeroconf = Zeroconf()

    async def publish_service(self, data: ContainerStatus):
        try:
            service_info = self.services[data.c_id]
        except KeyError:
            service_info = self._create_service_info(data)
        try:
            self.zeroconf.register_service(service_info)
            log.info("Service %s:%s published", data.c_id, service_info.port)
        except NonUniqueNameException:
            log.warning("Service name '%s' is already registered!", service_info.name )

    async def unpublish_service(self, container_id: str):
        try:
            service_info = self.services[container_id]
            self.zeroconf.unregister_service(service_info)
            log.info("Service %s:%s unpublished", container_id, service_info.port)
        except KeyError:
            pass

    def _create_service_info(self, data: ContainerStatus) -> ServiceInfo:
        service_type = f"_{self._get_valid_service_name(data.c_id)}._{data.ports[0][1]}.local."
        service_name = f"{data.name}.{service_type}"
        service_port = data.ports[0][0]

        service_info = ServiceInfo(
            type_=service_type, name=service_name,
            port=service_port, server=self.server
        )
        self.services[data.c_id] = service_info
        return service_info


    def _get_valid_service_name(self, name: str) -> str:
        name = name.replace(" ", "_")
        if len(name) > 15:
            name = name.replace("_", "")
        if len(name) > 15:
            name = name.replace("-", "")
        if len(name) > 15:
            name = name[0:15]
        return name


if __name__ == "__main__":
    cmd_args = argparse.ArgumentParser(formatter_class=argparse.ArgumentDefaultsHelpFormatter)

    cmd_args.add_argument("-s", "--socket_path",
                      default="/run/container-management/container-management.sock",
                      help="path to the Eclipse Kanto container management")
    cmd_args.add_argument("-i", "--interval",
                      default="2", type=int,
                      help="update interval for the service sync in seconds")
    cmd_args.add_argument("-d", "--debug",
                      default=False, action="store_true",
                      help="switch log level to debug mode")

    config = cmd_args.parse_args()
    logging.basicConfig(level=logging.DEBUG if config.debug else logging.INFO )

    app = KantocmZeroconf(socket_path=config.socket_path, interval=config.interval)
    asyncio.run(app.run())
