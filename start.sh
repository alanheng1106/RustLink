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

# 2. Check or install portable Ninja (replaces missing 'make')
if ! command -v ninja &> /dev/null; then
    echo "[-] ninja not found. Downloading Ninja build tool..."
    curl -sSL "https://github.com/ninja-build/ninja/releases/download/v1.12.1/ninja-linux.zip" -o "/tmp/ninja.zip"
    mkdir -p /tmp/ninja_extract
    (cd /tmp/ninja_extract && cmake -E tar xf /tmp/ninja.zip && mv ninja "$BIN_DIR/ninja")
    chmod +x "$BIN_DIR/ninja"
    ln -sf "$BIN_DIR/ninja" "$BIN_DIR/make"
    rm -rf /tmp/ninja_extract /tmp/ninja.zip
    echo "[+] Ninja installed successfully!"
fi

export CMAKE_GENERATOR="Ninja"
export CMAKE_MAKE_PROGRAM="$BIN_DIR/ninja"

# Clean any stale build dir from previously failed Make generator runs
rm -rf target/release/build/libopus_sys* target/debug/build/libopus_sys* 2>/dev/null || true

# 3. Check c++ compiler alias
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

# 4. Check or download standalone yt-dlp binary (with embedded Python)
if [ ! -f "$BIN_DIR/yt-dlp" ] || head -n 1 "$BIN_DIR/yt-dlp" 2>/dev/null | grep -q "python"; then
    echo "[-] Downloading standalone yt-dlp_linux binary (embedded Python runtime)..."
    curl -sSL "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux" -o "$BIN_DIR/yt-dlp"
    chmod +x "$BIN_DIR/yt-dlp"
    echo "[+] Standalone yt-dlp installed successfully!"
fi

# 5. Check or download standalone Deno JS runtime (required by yt-dlp for YouTube signature cipher)
if ! command -v deno &> /dev/null; then
    echo "[-] deno not found. Downloading standalone Deno JS runtime for YouTube signature cipher..."
    curl -sSL "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip" -o "/tmp/deno.zip"
    mkdir -p /tmp/deno_extract
    (cd /tmp/deno_extract && cmake -E tar xf /tmp/deno.zip && mv deno "$BIN_DIR/deno")
    chmod +x "$BIN_DIR/deno"
    rm -rf /tmp/deno_extract /tmp/deno.zip
    echo "[+] Deno JS runtime installed successfully!"
fi

echo "=== [RustLink] Starting Cargo build & run ==="
exec cargo run --release
