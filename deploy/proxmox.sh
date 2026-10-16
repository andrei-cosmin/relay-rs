#!/bin/sh
set -eu

repository="${RELAY_REPO:-andrei-cosmin/relay-rs}"
latest_release="https://github.com/$repository/releases/latest/download"
container_id="${RELAY_CTID:-}"
disk_storage="${RELAY_STORAGE:-}"
network_bridge="${RELAY_BRIDGE:-vmbr0}"
template_storage="${RELAY_TEMPLATE_STORAGE:-local}"

main() {
  require_proxmox_host
  choose_container_id
  choose_disk_storage
  download_debian_template
  create_container
  wait_for_network
  install_relay_in_container
  print_summary
}

require_proxmox_host() {
  [ "$(id -u)" -eq 0 ] || fail "run this as root on the Proxmox host"
  for tool in pct pveam pvesh pvesm; do
    command -v "$tool" >/dev/null || fail "$tool not found; run this on the Proxmox host"
  done
}

choose_container_id() {
  [ -n "$container_id" ] || container_id="$(pvesh get /cluster/nextid)"
}

choose_disk_storage() {
  [ -n "$disk_storage" ] || disk_storage="$(pvesm status -content rootdir | awk 'NR > 1 && $3 == "active" { print $1; exit }')"
  [ -n "$disk_storage" ] || fail "no active storage for container disks; set RELAY_STORAGE"
}

download_debian_template() {
  architecture="$(dpkg --print-architecture)"
  say "finding the latest Debian 13 template for $architecture"
  pveam update >/dev/null
  template="$(pveam available --section system | awk '{ print $2 }' | grep "^debian-13-standard_.*_$architecture\.tar" | sort -V | tail -1)"
  [ -n "$template" ] || fail "no Debian 13 template is available for $architecture"
  if ! pveam list "$template_storage" | grep -q "$template"; then
    say "downloading $template"
    pveam download "$template_storage" "$template" >/dev/null
  fi
}

create_container() {
  say "creating container $container_id on $disk_storage"
  pct create "$container_id" "$template_storage:vztmpl/$template" \
    --hostname relay \
    --unprivileged 1 \
    --features nesting=1 \
    --cores 1 \
    --memory 256 \
    --rootfs "$disk_storage:1" \
    --net0 "name=eth0,bridge=$network_bridge,ip=dhcp" \
    --onboot 1
  pct start "$container_id"
}

wait_for_network() {
  say "waiting for the network"
  address=""
  seconds=0
  while [ -z "$address" ]; do
    [ "$seconds" -lt 60 ] || fail "container $container_id got no address; check DHCP on $network_bridge"
    sleep 1
    seconds=$((seconds + 1))
    address="$(pct exec "$container_id" -- hostname -I 2>/dev/null | tr ' ' '\n' | grep -m1 -E '^[0-9]+\.' || true)"
  done
}

install_relay_in_container() {
  say "installing relay"
  pct exec "$container_id" -- env LC_ALL=C.UTF-8 DEBIAN_FRONTEND=noninteractive \
    sh -c "apt-get update -qq && apt-get install -y -qq curl ca-certificates >/dev/null && apt-get clean"
  pct exec "$container_id" -- env LC_ALL=C.UTF-8 \
    sh -c "curl -fsSL $latest_release/install.sh | sh -s -- server"
}

print_summary() {
  say "relay is running in container $container_id"
  say "  page:    http://$address:8080/"
  say "  logs:    pct exec $container_id -- journalctl -u relay -f"
  say "  update:  pct exec $container_id -- sh -c 'curl -fsSL $latest_release/install.sh | sh -s -- server'"
}

say() {
  printf '\033[1;33mrelay\033[0m %s\n' "$*"
}

fail() {
  printf '\033[1;31mrelay\033[0m %s\n' "$*" >&2
  exit 1
}

main "$@"
