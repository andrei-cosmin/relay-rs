#!/bin/sh
set -eu

main() {
  mode="${1:-}"
  case "$mode" in
    server) provision_for_server ;;
    standalone) provision_for_standalone_build ;;
    *) fail "usage: provision-tools.sh server | standalone" ;;
  esac
}

provision_for_server() {
  require_root "server mode installs into /usr/local/bin"
  require_command curl
  detect_this_machine
  make_scratch_folder
  mkdir -p /usr/local/bin
  install_cloudflared_to /usr/local/bin/cloudflared
  install_wireproxy_to /usr/local/bin/wireproxy
  say "tools installed in /usr/local/bin"
}

provision_for_standalone_build() {
  require_command curl
  require_command rustc
  detect_build_target
  make_scratch_folder
  bundle_folder="$(cd "$(dirname "$0")/.." && pwd)/relay-app/bin"
  mkdir -p "$bundle_folder"
  install_cloudflared_to "$bundle_folder/cloudflared-$build_target"
  install_wireproxy_to "$bundle_folder/wireproxy-$build_target"
  say "tools ready in relay-app/bin for $build_target"
}

detect_this_machine() {
  case "$(uname -s)" in
    Linux) system="linux" ;;
    Darwin) system="darwin" ;;
    *) fail "unsupported system: $(uname -s)" ;;
  esac
  case "$(uname -m)" in
    x86_64|amd64) processor="amd64" ;;
    aarch64|arm64) processor="arm64" ;;
    *) fail "unsupported processor: $(uname -m)" ;;
  esac
}

detect_build_target() {
  build_target="$(rustc -vV | sed -n 's/^host: //p')"
  case "$build_target" in
    aarch64-apple-darwin) system="darwin"; processor="arm64" ;;
    x86_64-apple-darwin) system="darwin"; processor="amd64" ;;
    aarch64-unknown-linux-gnu) system="linux"; processor="arm64" ;;
    x86_64-unknown-linux-gnu) system="linux"; processor="amd64" ;;
    *) fail "unsupported build target: $build_target" ;;
  esac
}

install_cloudflared_to() {
  destination="$1"
  say "downloading cloudflared for $system-$processor"
  releases="https://github.com/cloudflare/cloudflared/releases/latest/download"
  if [ "$system" = "linux" ]; then
    curl -fsSL "$releases/cloudflared-linux-$processor" -o "$scratch/cloudflared"
  else
    curl -fsSL "$releases/cloudflared-darwin-$processor.tgz" -o "$scratch/cloudflared.tgz"
    tar -C "$scratch" -xzf "$scratch/cloudflared.tgz"
  fi
  install_executable "$scratch/cloudflared" "$destination"
}

install_wireproxy_to() {
  destination="$1"
  say "downloading wireproxy for $system-$processor"
  releases="https://github.com/windtf/wireproxy/releases/latest/download"
  curl -fsSL "$releases/wireproxy_${system}_${processor}.tar.gz" -o "$scratch/wireproxy.tgz"
  tar -C "$scratch" -xzf "$scratch/wireproxy.tgz"
  install_executable "$scratch/wireproxy" "$destination"
}

install_executable() {
  source_file="$1"
  destination="$2"
  cp "$source_file" "$destination.new"
  chmod 755 "$destination.new"
  mv -f "$destination.new" "$destination"
}

make_scratch_folder() {
  scratch="$(mktemp -d)"
  trap 'rm -rf "$scratch"' EXIT
}

require_root() {
  [ "$(id -u)" -eq 0 ] || fail "$1 and must run as root"
}

require_command() {
  command -v "$1" >/dev/null || fail "$1 is required"
}

say() {
  printf '\033[1;33mrelay\033[0m %s\n' "$*"
}

fail() {
  printf '\033[1;31mrelay\033[0m %s\n' "$*" >&2
  exit 1
}

main "$@"
