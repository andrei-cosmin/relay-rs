# relay-rs

[![Tests](https://github.com/andrei-cosmin/relay-rs/actions/workflows/test.yml/badge.svg)](https://github.com/andrei-cosmin/relay-rs/actions/workflows/test.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A reverse proxy with a built-in admin page, published through a Cloudflare tunnel.

## Features

- **Path routing**: each path prefix forwards to its own upstream.
- **Headers**: add or strip headers per target.
- **Live monitor**: every request appears in the admin page as it happens.
- **Tunnel**: a quick `trycloudflare.com` address or your own domain.
- **Per-target VPN**: each target can send its traffic through its own WireGuard tunnel.
- **Two modes**: a server with a web admin page, or a desktop app.

## Installation

| Mode | Runs on | Command |
|---|---|---|
| Server | Linux with systemd, or macOS | `curl -fsSL https://github.com/andrei-cosmin/relay-rs/releases/latest/download/install.sh \| sudo sh -s -- server` |
| Proxmox | the Proxmox host, as root | `curl -fsSL https://github.com/andrei-cosmin/relay-rs/releases/latest/download/proxmox.sh \| sh` |
| Desktop app | macOS or Linux, without `sudo` | `curl -fsSL https://github.com/andrei-cosmin/relay-rs/releases/latest/download/install.sh \| sh -s -- standalone` |

All three support x86_64 and arm64.

### Server

The script installs relay to `/opt/relay`, installs `cloudflared` and `wireproxy` to `/usr/local/bin`, and starts relay
as a systemd service on Linux or a launchd daemon on macOS.

| | |
|---|---|
| Admin page | `http://<host>:8080/` |
| Logs | `journalctl -u relay -f` on Linux, `/opt/relay/relay.log` on macOS |
| Update | run the install command again |

### Proxmox

The script creates a container, installs relay in it as a server, and prints the admin page address.

| Container | |
|---|---|
| System | Debian 13, unprivileged |
| Features | nesting, the Proxmox default for unprivileged containers |
| CPU | 1 core |
| Memory | 256 MB |
| Disk | 1 GB |
| Network | DHCP, starts on boot |

| Variable | Default |
|---|---|
| `RELAY_CTID` | the next free ID |
| `RELAY_STORAGE` | the first active storage for container disks |
| `RELAY_BRIDGE` | `vmbr0` |
| `RELAY_TEMPLATE_STORAGE` | `local` |

To update, run in the host shell, with your container ID:

```sh
pct exec <id> -- sh -c 'curl -fsSL https://github.com/andrei-cosmin/relay-rs/releases/latest/download/install.sh | sh -s -- server'
```

### Desktop app

The script installs `Relay.app` to `/Applications` on macOS, or the AppImage to `~/.local/bin/relay` on Linux. The app
includes `cloudflared` and `wireproxy`. The same files are on the
[latest release](https://github.com/andrei-cosmin/relay-rs/releases/latest) page. To update, run the install command
again.

## Configuration

Settings are edited on the admin page and stored in `relay.ron` in the data folder. Tunnel changes apply on restart.

| Mode | Data folder |
|---|---|
| Server | `/opt/relay/data`, or `data/` in the working directory when run by hand |
| Desktop, macOS | `~/Library/Application Support/Relay-RS` |
| Desktop, Linux | `~/.local/share/Relay-RS` |

The server listens on the address from the `IP` and `PORT` environment variables, which the installed service sets to
`0.0.0.0` and `8080`.

### Tunnel

| Mode | Behavior |
|---|---|
| Quick, the default | A free random `trycloudflare.com` address that changes on every restart. |
| Own domain | A Cloudflare tunnel on your domain. Create the tunnel in the Cloudflare dashboard under Zero Trust, Networks, Tunnels, point a public hostname at the address shown on the Settings page, and paste the tunnel token there. |
| Off | No tunnel. The proxy is reachable on the local network only. |

The active tunnel address is shown in the admin page header and printed in the log.

## Uninstall

| Mode | Command |
|---|---|
| Server, Linux | `sudo systemctl disable --now relay && sudo rm -rf /opt/relay /etc/systemd/system/relay.service && sudo userdel relay` |
| Server, macOS | `sudo launchctl bootout system /Library/LaunchDaemons/dev.relay.plist && sudo rm -rf /opt/relay /Library/LaunchDaemons/dev.relay.plist` |
| Proxmox | `pct stop <id> && pct destroy <id>` |
| Desktop app | delete `Relay.app` or `~/.local/bin/relay`, and the data folder |

The server install leaves `cloudflared` and `wireproxy` in `/usr/local/bin`; delete them too if nothing else uses them.

## Building from source

### Requirements

```sh
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli --version 0.7.10 --locked
```

A Linux desktop build also needs the system webview libraries. On Debian and Ubuntu:

```sh
sudo apt-get install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev lld
```

### Server

```sh
dx bundle --package relay-app --platform web --fullstack --release
cd target/dx/relay/release/web
./server
```

The output contains the `server` binary and the `public/` folder it serves. Tunnels and VPNs need `cloudflared` and
`wireproxy` on `PATH`, which `sudo sh deploy/provision-tools.sh server` installs. Without them relay starts with both
features unavailable.

### Desktop app

```sh
sh deploy/provision-tools.sh standalone
dx bundle --package relay-app --desktop --release --package-types macos --package-types dmg
```

`provision-tools.sh standalone` downloads `cloudflared` and `wireproxy` for the build machine, and the bundle includes
them. On Linux, use `--package-types appimage`.

### Tests

```sh
cargo test --workspace --features relay-app/server
cargo test --workspace --features relay-app/standalone
```

## License

[MIT](LICENSE)
