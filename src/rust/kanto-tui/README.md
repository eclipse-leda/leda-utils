# Kanto Container Management TUI

A simple ncurses-TUI for kanto-cm that allows for easier management of deployed containers. Supports mouse and keyboard interaction. To get started:

```bash
    kantui --help
```

![Screenshot](misc/kantocmcurses-ss.png)

NCurses controls the stdout and cleans up on exit/crash. Thus to capture (if needed) error messages:

```bash
RUST_BACKTRACE=1 kantui 2> stderr.log
```

## Building

- Ensure git submodules exist by running in workspace root:
    ```
    git submodule init
    git submodule update
    ```
- Copy default config file to location:
    ```
    cd src/rust/kanto-tui
    sudo mkdir -p /etc/kantui/
    sudo cp kantui_conf.toml /etc/kantui/kantui_conf.toml
    ```

## Testing
- Run unit tests:
    ```
    cargo test
    ```
- Start Kanto Container Management
- Run kantui:
    ```
    # Kantui must be run as root
    sudo cargo run
    ```
