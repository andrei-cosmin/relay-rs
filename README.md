# relay-rs

A small reverse proxy with a web page to run it. It exposes itself through a Cloudflare tunnel, forwards each path
prefix to the upstream you choose, adds or strips headers per target, shows every request as it happens, and can send
each target out through its own WireGuard VPN.

## Install

Linux (Debian, Ubuntu, a Proxmox LXC) or macOS, x86_64 or arm64:

```
curl -fsSL https://github.com/andrei-cosmin/relay-rs/releases/latest/download/install.sh | sudo sh
```

It installs the relay, `cloudflared` and `wireproxy`, and starts the relay as a service. On its first start the relay
writes its default config. The page is on port 8080: the service file sets `IP=0.0.0.0` and `PORT=8080`, the two
variables the Dioxus server reads for its address. The tunnel URL is printed in the log and shown in the page header.

Run the same command again to update.

## Tunnel

Chosen on the Settings page, applied on restart:

- **Quick**, the default: a free random `trycloudflare.com` address that changes on every restart.
- **Own domain**: create a tunnel in the Cloudflare dashboard (Zero Trust, Networks, Tunnels), add a public hostname on
  your domain pointing at the relay's listen address, and paste the tunnel token into the page. The Settings page shows
  the exact address to use. The header shows your hostname once connected.
- **Off**: no tunnel; the proxy is reachable only on the network.

## Proxmox LXC

The relay runs in an unprivileged container. No extra privileges, devices or nesting are required.

| Resource | Recommended |
|---|---|
| CPU | 1 core |
| Memory | 512 MB |
| Disk | 2 GB |

The commands below run on the Proxmox host. Replace `200` with a free container ID, `local-lvm` with your container
storage and `vmbr0` with your bridge.

**1. Download a Debian template**

```
pveam update
pveam available --section system | grep debian
pveam download local <template>
```

**2. Create and start the container**

```
pct create 200 local:vztmpl/<template> \
  --hostname relay \
  --unprivileged 1 \
  --cores 1 \
  --memory 512 \
  --rootfs local-lvm:2 \
  --net0 name=eth0,bridge=vmbr0,ip=dhcp \
  --onboot 1
pct start 200
```

**3. Install the relay**

```
pct exec 200 -- sh -c 'apt-get update && apt-get install -y curl && curl -fsSL https://github.com/andrei-cosmin/relay-rs/releases/latest/download/install.sh | sh'
```

**4. Open the admin page**

```
pct exec 200 -- hostname -I
```

The page is served at `http://<container-address>:8080/`.

The same setup can be done from the Proxmox web interface: create an unprivileged container from the Debian template
with DHCP networking, start it, and run the quoted part of step 3 in its console.

Logs are available with `pct exec 200 -- journalctl -u relay -f`.

## Build from source

Relay is a Dioxus fullstack app. One build produces the server binary and the admin page, compiled to WebAssembly.

**Requirements:** stable Rust and the Dioxus CLI.

```
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli --version 0.7.10 --locked
```

**Build**

```
dx bundle --package relay-server --platform web --fullstack --release
```

The output is `target/dx/relay/release/web/`: the `server` binary and the `public/` folder it serves.

**Run**

```
cd target/dx/relay/release/web
./server
```

On first start it creates `data/` with the default configuration in the working directory. Without `cloudflared` or
`wireproxy` on `PATH` it still starts, with the tunnel and VPN features unavailable.

**Tests**

```
cargo test --workspace --features relay-server/server
```

## License

MIT, see `LICENSE`.
