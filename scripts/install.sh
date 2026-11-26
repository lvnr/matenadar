#!/bin/sh
# Installs the prebuilt `matenadar` CLI binary from GitHub Releases.
#
#   curl -fsSL https://github.com/lvnr/matenadar/releases/latest/download/install.sh | sh
#
# Environment variables:
#   MATENADAR_VERSION     Tag or version to install (default: latest, e.g. v0.1.0)
#   MATENADAR_INSTALL_DIR Destination directory (default: $HOME/.local/bin)
set -eu

REPO="lvnr/matenadar"
INSTALL_DIR="${MATENADAR_INSTALL_DIR:-$HOME/.local/bin}"

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "matenadar installer: missing required command: $1" >&2
    exit 1
  fi
}

need curl
need tar

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin) os_part="apple-darwin" ;;
  Linux)  os_part="unknown-linux-gnu" ;;
  *)
    echo "matenadar installer: unsupported OS: $OS" >&2
    exit 1
    ;;
esac

case "$ARCH" in
  arm64|aarch64) arch_part="aarch64" ;;
  x86_64|amd64)  arch_part="x86_64" ;;
  *)
    echo "matenadar installer: unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

TARGET="${arch_part}-${os_part}"

if [ -n "${MATENADAR_VERSION:-}" ]; then
  case "$MATENADAR_VERSION" in
    v*) TAG="$MATENADAR_VERSION" ;;
    *)  TAG="v$MATENADAR_VERSION" ;;
  esac
else
  LATEST_URL="$(curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest")"
  TAG="${LATEST_URL##*/}"
  if [ -z "$TAG" ] || [ "$TAG" = "releases" ] || [ "$TAG" = "latest" ]; then
    echo "matenadar installer: could not determine latest release tag" >&2
    exit 1
  fi
fi

ARCHIVE="matenadar-${TAG}-${TARGET}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/${TAG}"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT INT TERM

echo "Downloading matenadar ${TAG} (${TARGET})..."
curl -fsSL "${BASE_URL}/${ARCHIVE}" -o "${TMPDIR}/${ARCHIVE}"
curl -fsSL "${BASE_URL}/SHA256SUMS.txt" -o "${TMPDIR}/SHA256SUMS.txt"

EXPECTED="$(awk -v f="$ARCHIVE" '$2 == f || $2 == ("*" f) { print $1; exit }' "${TMPDIR}/SHA256SUMS.txt")"
if [ -z "$EXPECTED" ]; then
  echo "matenadar installer: checksum for ${ARCHIVE} not found in SHA256SUMS.txt" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL="$(sha256sum "${TMPDIR}/${ARCHIVE}" | awk '{ print $1 }')"
elif command -v shasum >/dev/null 2>&1; then
  ACTUAL="$(shasum -a 256 "${TMPDIR}/${ARCHIVE}" | awk '{ print $1 }')"
else
  echo "matenadar installer: neither sha256sum nor shasum found to verify download" >&2
  exit 1
fi

if [ "$ACTUAL" != "$EXPECTED" ]; then
  echo "matenadar installer: checksum mismatch for ${ARCHIVE}" >&2
  echo "  expected: ${EXPECTED}" >&2
  echo "  actual:   ${ACTUAL}" >&2
  exit 1
fi

tar -xzf "${TMPDIR}/${ARCHIVE}" -C "$TMPDIR"
mkdir -p "$INSTALL_DIR"
mv "${TMPDIR}/matenadar" "${INSTALL_DIR}/matenadar"
chmod 755 "${INSTALL_DIR}/matenadar"

echo "Installed $("${INSTALL_DIR}/matenadar" --version) to ${INSTALL_DIR}/matenadar"
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo ""
    echo "Add ${INSTALL_DIR} to your PATH:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    ;;
esac
