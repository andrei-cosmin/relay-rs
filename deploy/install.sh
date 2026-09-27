#!/bin/sh
set -eu

REPO="${RELAY_REPO:-andrei-cosmin/relay-rs}"
HOME_DIR="/opt/relay"
BIN_DIR="/usr/local/bin"

say() { printf '\033[1;33mrelay\033[0m %s\n' "$*"; }
die() { printf '\033[1;31mrelay\033[0m %s\n' "$*" >&2; exit 1; }
place() { cp "$1" "$2.new"; chmod 755 "$2.new"; mv -f "$2.new" "$2"; }

[ "$(id -u)" -eq 0 ] || die "run as root: curl -fsSL <url> | sudo sh"

os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Linux) platform="linux" ;;
  Darwin) platform="macos" ;;
  *) die "unsupported os: $os" ;;
esac
case "$arch" in
  x86_64|amd64) cpu="x86_64"; cf_cpu="amd64"; wp_cpu="amd64" ;;
  aarch64|arm64) cpu="aarch64"; cf_cpu="arm64"; wp_cpu="arm64" ;;
  *) die "unsupported cpu: $arch" ;;
esac

command -v curl >/dev/null || die "curl is required"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

say "downloading relay for $platform-$cpu"
curl -fsSL "https://github.com/$REPO/releases/latest/download/relay-$platform-$cpu.tar.gz" -o "$tmp/relay.tar.gz"
mkdir -p "$HOME_DIR"
tar -C "$tmp" -xzf "$tmp/relay.tar.gz"
place "$tmp/relay/relay" "$HOME_DIR/relay"
rm -rf "$HOME_DIR/relay-ui" "$HOME_DIR/public" "$HOME_DIR/deploy"
cp -R "$tmp/relay/public" "$HOME_DIR/public"
cp -R "$tmp/relay/deploy" "$HOME_DIR/deploy"

say "downloading cloudflared"
if [ "$platform" = "linux" ]; then
  curl -fsSL "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-$cf_cpu" -o "$tmp/cloudflared"
else
  curl -fsSL "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-darwin-$cf_cpu.tgz" -o "$tmp/cloudflared.tgz"
  tar -C "$tmp" -xzf "$tmp/cloudflared.tgz"
fi
place "$tmp/cloudflared" "$BIN_DIR/cloudflared"

say "downloading wireproxy"
wp_os="$platform"
[ "$platform" = "macos" ] && wp_os="darwin"
curl -fsSL "https://github.com/windtf/wireproxy/releases/latest/download/wireproxy_${wp_os}_${wp_cpu}.tar.gz" -o "$tmp/wireproxy.tgz"
tar -C "$tmp" -xzf "$tmp/wireproxy.tgz"
place "$tmp/wireproxy" "$BIN_DIR/wireproxy"

mkdir -p "$HOME_DIR/data"
chmod 700 "$HOME_DIR/data"

if [ "$platform" = "linux" ]; then
  id relay >/dev/null 2>&1 || useradd --system --home "$HOME_DIR" --shell /usr/sbin/nologin relay
  chown -R relay:relay "$HOME_DIR"
  cp "$HOME_DIR/deploy/relay.service" /etc/systemd/system/relay.service
  systemctl daemon-reload
  systemctl enable relay >/dev/null
  systemctl restart relay
  say "installed; the relay runs as a systemd service"
  say "  logs:   journalctl -u relay -f"
  say "  page:   http://$(hostname -I 2>/dev/null | awk '{print $1}'):8080/"
else
  cp "$HOME_DIR/deploy/relay.plist" /Library/LaunchDaemons/dev.relay.plist
  launchctl bootout system /Library/LaunchDaemons/dev.relay.plist 2>/dev/null || true
  launchctl bootstrap system /Library/LaunchDaemons/dev.relay.plist
  say "installed; the relay runs as a launchd daemon"
  say "  logs:   tail -f /opt/relay/relay.log"
  say "  page:   http://localhost:8080/"
fi
