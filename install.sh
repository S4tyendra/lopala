#!/usr/bin/env bash
set -euo pipefail

REPO="s4tyendra/lopala"
BINARY_DEST="/usr/bin/lopala"

# ── 1. OS check ───────────────────────────────────────────────────────────────
OS=$(uname -s)
if [ "$OS" != "Linux" ]; then
    echo "Error: Lopala only supports Linux (detected: $OS)."
    exit 1
fi

# ── 2. Architecture detection ─────────────────────────────────────────────────
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)          BINARY_NAME="lopala-linux-x64" ;;
    aarch64|arm64)   BINARY_NAME="lopala-linux-arm64" ;;
    *)               echo "Error: Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "→ OS: $OS  |  Architecture: $ARCH"

# ── 3. Fetch latest release tag from GitHub API ───────────────────────────────
echo "→ Checking latest release..."
API_RESPONSE=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest") || {
    echo "Error: Could not reach GitHub API. Check your internet connection."
    exit 1
}

LATEST_TAG=$(echo "$API_RESPONSE" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
if [ -z "$LATEST_TAG" ]; then
    echo "Error: Could not parse latest release tag from GitHub API response."
    exit 1
fi

echo "→ Latest release: $LATEST_TAG"

# ── 4. Compare with installed version ─────────────────────────────────────────
CURRENT_VER="none"
if [ -x "$BINARY_DEST" ]; then
    RAW=$("$BINARY_DEST" --version 2>/dev/null || true)
    # `lopala 0.0.1` → strip leading 'v' from LATEST_TAG to compare apples-to-apples
    CURRENT_VER="v$(echo "$RAW" | awk '{print $2}')"
fi

echo "→ Installed version: $CURRENT_VER"

if [ "$CURRENT_VER" = "$LATEST_TAG" ]; then
    echo "✅ Already up to date ($LATEST_TAG). Skipping download."
else
    # ── 5. Download ───────────────────────────────────────────────────────────
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/$BINARY_NAME"
    echo "→ Downloading $BINARY_NAME from $DOWNLOAD_URL ..."

    TMP_FILE=$(mktemp /tmp/lopala.XXXXXX)
    # Clean up temp file on exit (success or failure)
    trap 'rm -f "$TMP_FILE"' EXIT

    if ! curl -fSL "$DOWNLOAD_URL" -o "$TMP_FILE"; then
        echo "Error: Download failed (URL: $DOWNLOAD_URL)"
        exit 1
    fi

    # Sanity check — binary must be more than 1 MB to be valid
    FILE_SIZE=$(stat -c%s "$TMP_FILE" 2>/dev/null || stat -f%z "$TMP_FILE")
    if [ "$FILE_SIZE" -lt 1000000 ]; then
        echo "Error: Downloaded file is suspiciously small ($FILE_SIZE bytes). Aborting."
        exit 1
    fi

    chmod +x "$TMP_FILE"

    # ── 6. Install ───────────────────────────────────────────────────────────
    echo "→ Installing to $BINARY_DEST (requires sudo)..."
    sudo mv "$TMP_FILE" "$BINARY_DEST"
    sudo chmod +x "$BINARY_DEST"
    echo "✅ Lopala $LATEST_TAG installed successfully."
fi

# ── 7. Dependency audit ───────────────────────────────────────────────────────
echo ""
echo "→ Checking dependencies..."

if command -v cloudflared &> /dev/null; then
    echo "✅ 'cloudflared' is installed."
else
    echo "⚠️  'cloudflared' not found."
    echo "   That's fine — Lopala will auto-download it when you run with --tunnel."
fi

if command -v rg &> /dev/null; then
    echo "✅ 'ripgrep' (rg) is installed."
else
    echo "❌ 'ripgrep' (rg) is missing. System search will not work."
    echo "   Install it via your package manager:"
    echo "     Ubuntu/Debian : sudo apt install ripgrep"
    echo "     Fedora/RHEL   : sudo dnf install ripgrep"
    echo "     Arch Linux    : sudo pacman -S ripgrep"
    echo "     Alpine        : sudo apk add ripgrep"
    echo "     openSUSE      : sudo zypper install ripgrep"
fi

echo ""
echo "✨ Done. Run: lopala --port 8080"
