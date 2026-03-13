#!/bin/bash
set -e

REPO="jimmyalcala/ftp_downloader"
BINARY="ftp_downloader"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC} $1"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        case "$ARCH" in
            x86_64) ASSET="ftp_downloader-linux-x86_64.tar.gz" ;;
            *) error "Unsupported architecture: $ARCH" ;;
        esac
        INSTALL_DIR="/usr/local/bin"
        ;;
    Darwin)
        case "$ARCH" in
            x86_64)  ASSET="ftp_downloader-macos-x86_64.tar.gz" ;;
            arm64)   ASSET="ftp_downloader-macos-aarch64.tar.gz" ;;
            *) error "Unsupported architecture: $ARCH" ;;
        esac
        INSTALL_DIR="/usr/local/bin"
        ;;
    *)
        error "Unsupported OS: $OS. For Windows, download the .zip from GitHub Releases."
        ;;
esac

# Get latest release tag
info "Fetching latest release..."
TAG=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | cut -d'"' -f4)

if [ -z "$TAG" ]; then
    error "Could not determine latest release. Check https://github.com/${REPO}/releases"
fi

info "Latest version: $TAG"

# Download
URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"
TMPDIR=$(mktemp -d)
info "Downloading $ASSET..."

if ! curl -sL "$URL" -o "${TMPDIR}/${ASSET}"; then
    rm -rf "$TMPDIR"
    error "Failed to download from $URL"
fi

# Extract
info "Extracting..."
tar -xzf "${TMPDIR}/${ASSET}" -C "$TMPDIR"

# Install
info "Installing to $INSTALL_DIR..."
if [ -w "$INSTALL_DIR" ]; then
    mv "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
else
    sudo mv "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
fi
chmod +x "${INSTALL_DIR}/${BINARY}"

# Copy example config if not exists
if [ ! -f "config.toml" ] && [ -f "${TMPDIR}/config.toml.example" ]; then
    cp "${TMPDIR}/config.toml.example" ./config.toml.example
    info "Example config copied to ./config.toml.example"
fi

# Cleanup
rm -rf "$TMPDIR"

info "Successfully installed ${BINARY} ${TAG} to ${INSTALL_DIR}/${BINARY}"
echo ""
echo "Usage:"
echo "  ${BINARY}                  # Run with config.toml in current directory"
echo "  ${BINARY} my_config.toml   # Run with custom config"
echo "  ${BINARY} --nogui          # Run without TUI"
