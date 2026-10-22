#!/usr/bin/env bash
set -e

echo "=== [RustLink] Checking environment in container ==="

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
export PATH="$HOME/.local/cmake/bin:$BIN_DIR:$HOME/.cargo/bin:$PATH"

# 1. Check or install portable CMake (no root required)
if ! command -v cmake &> /dev/null; then
    echo "[-] cmake not found. Downloading portable standalone CMake..."
    CMAKE_VERSION="3.30.5"
    CMAKE_TAR="cmake-${CMAKE_VERSION}-linux-x86_64.tar.gz"
    CMAKE_URL="https://github.com/Kitware/CMake/releases/download/v${CMAKE_VERSION}/${CMAKE_TAR}"
    
    mkdir -p "$HOME/.local/cmake"
    curl -sSL "$CMAKE_URL" | tar -xz -C "$HOME/.local/cmake" --strip-components=1
    ln -sf "$HOME/.local/cmake/bin/cmake" "$BIN_DIR/cmake"
    echo "[+] Portable CMake installed successfully!"
fi

# 2. Check c++ compiler alias
if ! command -v c++ &> /dev/null; then
    echo "[-] c++ alias not found. Setting up compiler alias..."
    if command -v g++ &> /dev/null; then
        ln -sf "$(command -v g++)" "$BIN_DIR/c++"
    elif command -v gcc &> /dev/null; then
        ln -sf "$(command -v gcc)" "$BIN_DIR/c++"
    elif command -v cc &> /dev/null; then
        ln -sf "$(command -v cc)" "$BIN_DIR/c++"
    elif [ -f /usr/bin/cc ]; then
        ln -sf /usr/bin/cc "$BIN_DIR/c++"
    fi
    echo "[+] c++ compiler alias ready: $(command -v c++ || echo 'fallback')"
fi

# 3. Check or download portable yt-dlp
if ! command -v yt-dlp &> /dev/null; then
    echo "[-] yt-dlp not found. Downloading standalone yt-dlp..."
    curl -sSL "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp" -o "$BIN_DIR/yt-dlp"
    chmod +x "$BIN_DIR/yt-dlp"
    echo "[+] Standalone yt-dlp installed successfully!"
fi

echo "=== [RustLink] Starting Cargo build & run ==="
exec cargo run --release
