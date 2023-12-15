# Kanto Container Management Zeroconf Service

A system service, that monitors the Kanto Container Management and
publishes/unpublishes containers with ports mapped to the host system.

## Requirements:

The following python modules are required:

- grpcio
- zeroconf

Furthermore the compiled python protobuf files of the Kanto Container
Management are required.

On the Eclipse Leda distribution all the requirements are already
fulfilled and this service will be started on boot time via systemd.
