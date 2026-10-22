# RustLink

<p align="center">
  <strong>High-performance, lightweight, memory-efficient Lavalink v4 audio node written in Rust.</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#configuration">Configuration</a> •
  <a href="#client-integration">Client Integration</a> •
  <a href="#supported-sources">Supported Sources</a> •
  <a href="#中文说明">中文说明</a> •
  <a href="#license">License</a>
</p>

---

RustLink is a drop-in, lightweight alternative to standard Java-based Lavalink. Built on top of **Tokio**, **Axum**, and **Songbird**, RustLink provides a fully compatible Lavalink v4 REST and WebSocket protocol interface with near-instant startup times and significantly lower RAM consumption (~20MB vs ~1GB on the JVM).

## Features

- ⚡ **Ultra-Low Memory & High Performance**: Native Rust binary with zero GC pauses, using a fraction of the memory required by JVM-based Lavalink.
- 🔌 **Lavalink v4 Compatible**: Seamlessly supports standard Lavalink v4 client libraries (Node.js, Python, Java, Go, etc.).
- 🎵 **Rich Audio Sources**:
  - **YouTube**: Flat search (`ytsearch:`) and direct playback via standalone `yt-dlp` with QuickJS signature deciphering support.
  - **HTTP/HTTPS**: Direct stream URLs (MP3, OGG, FLAC, WAV, AAC).
  - **Local Files**: Local audio files via file paths or `file://` URLs.
  - **HLS**: Live audio streams using `.m3u8` playlists.
  - **Extensible**: Modular source architecture to easily plug in custom audio providers.
- 🔄 **Session Resuming & Session Reaper**: Automatic session reconnection recovery and race-free background garbage collection for abandoned sessions.
- 🚀 **Zero-Root Container Bootstrap**: Built-in `start.sh` launcher that can auto-install portable CMake, Ninja, yt-dlp, and QuickJS runtime on minimal Linux containers without root privileges.

---

## Quick Start

### Prerequisites

- **Rust**: 1.85+ (Edition 2024 supported)
- **CMake & C++ Compiler**: Required for compiling native audio bindings (libopus / songbird)
- **yt-dlp** (optional but recommended): For YouTube extraction

### 1. Clone & Build

```bash
git clone https://github.com/alanheng1106/RustLink.git
cd RustLink
cargo build --release
```

### 2. Run RustLink

Run with Cargo:
```bash
cargo run --release
```

Or run the release binary directly:
```bash
./target/release/rustlink
```

Or use the automated launcher (ideal for headless Linux / cloud environments):
```bash
chmod +x start.sh
./start.sh
```

Upon successful launch, RustLink will listen on `0.0.0.0:2333` by default.

---

## Configuration

RustLink automatically generates a default `application.yml` upon first run if one does not exist.

### `application.yml`

```yaml
# =======================================================
# RustLink Configuration File (Lavalink v4 Compatible)
# =======================================================

server:
  port: 2333
  address: 0.0.0.0
  password: "youshallnotpass"

# Supported audio sources
sources:
  youtube: true
  http: true
  local: true
  hls: true

# Logging level (trace, debug, info, warn, error)
logging:
  level: "info"
```

RustLink also supports standard Lavalink nested config blocks (`lavalink.server.password`, `lavalink.server.sources`).

### Environment Variables

Environment variables take priority over values in `application.yml`:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `RUSTLINK_SERVER_PORT` / `LAVALINK_SERVER_PORT` / `PORT` | Listening port | `2333` |
| `RUSTLINK_SERVER_HOST` / `LAVALINK_SERVER_HOST` | Host binding address | `0.0.0.0` |
| `RUSTLINK_SERVER_PASSWORD` / `LAVALINK_SERVER_PASSWORD` | Client authentication password | `youshallnotpass` |
| `RUST_LOG` | Tracing log filter expression | `info` |

---

## Client Integration

RustLink is compatible with standard Lavalink v4 client libraries. Connect your Discord bot using the following credentials:

- **Host**: `127.0.0.1` (or your server's IP / domain)
- **Port**: `2333`
- **Password**: `youshallnotpass` (or your configured password)
- **Secure**: `false` (or `true` if behind HTTPS/WSS reverse proxy)

### Node.js Example (`lavalink-client`)

```javascript
import { LavalinkManager } from "lavalink-client";

const client = new LavalinkManager({
  nodes: [
    {
      authorization: "youshallnotpass",
      host: "localhost",
      port: 2333,
      secure: false,
    },
  ],
  sendToShard: (guildId, payload) => {
    // Discord bot send to websocket implementation
  },
});
```

### Python Example (`wavelink`)

```python
import wavelink

node = wavelink.Node(
    uri="http://localhost:2333",
    password="youshallnotpass"
)
await wavelink.Pool.connect(nodes=[node], client=bot)
```

---

## Supported Sources

| Source | Prefix / Format | Example Identifier |
| :--- | :--- | :--- |
| **YouTube Search** | `ytsearch:<query>` | `ytsearch:never gonna give you up` |
| **YouTube Direct** | `https://www.youtube.com/...` or `https://youtu.be/...` | `https://www.youtube.com/watch?v=dQw4w9WgXcQ` |
| **HTTP Stream** | `http://` or `https://` | `https://icecast.example.com/stream.mp3` |
| **Local File** | `file://<path>` or filesystem path | `/var/music/song.mp3` |
| **HLS Stream** | URL containing `.m3u8` | `https://example.com/live/playlist.m3u8` |

---

## API Overview

RustLink implements the Lavalink v4 specification:

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `GET` | `/version` | Returns Lavalink version string (`4.0.0`) |
| `GET` | `/v4/info` | Node information, enabled sources, and plugins |
| `GET` | `/v4/stats` | Memory usage, CPU load, and player stats |
| `GET` | `/v4/loadtracks?identifier=...` | Resolves tracks or searches via audio sources |
| `POST` | `/v4/decode` | Decodes single Lavalink binary track string |
| `POST` | `/v4/decodetracks` | Decodes multiple track strings |
| `GET` | `/v4/websocket` | Lavalink v4 client WebSocket connection |
| `PATCH`| `/v4/sessions/{sessionId}` | Updates session configuration (resuming, timeout) |
| `GET` | `/v4/sessions/{sessionId}/players` | Retrieves list of players in session |
| `GET` | `/v4/sessions/{sessionId}/players/{guildId}` | Gets player state for a Discord guild |
| `PATCH`| `/v4/sessions/{sessionId}/players/{guildId}` | Updates player (play track, pause, volume, voice state) |
| `DELETE`| `/v4/sessions/{sessionId}/players/{guildId}`| Destroys guild player |

---

## Testing

Run the automated integration and smoke tests:

```bash
cargo test --test smoke_test
```

---

## 中文说明

RustLink 是一个基于 Rust 语言实现的轻量级、高性能 **Lavalink v4 音频节点**，用于替代原版的 Java JVM 实现。

### 为什么选择 RustLink？
- **极低内存开销**：冷启动与空闲仅占用约 15~30MB 内存，相比 Java 版动辄数百 MB 甚至上 GB 的堆内存大幅精简。
- **开箱即用兼容**：原生兼容 Lavalink v4 REST API 与 WebSocket 协议规范，无需修改现有的 Discord 机器人客户端代码。
- **丰富的音源支持**：
  - YouTube：支持关键词快速搜索（`ytsearch:`）与直接播放，搭配 `yt-dlp` 及 QuickJS JavaScript 签名解码引擎。
  - HTTP / HTTPS 网络音频直链（MP3, OGG, FLAC 等）。
  - 本地音频文件播放（支持文件路径或 `file://`）。
  - HLS 实时流播放（`.m3u8`）。
- **Session 恢复与自动回收**：支持 Session Resuming 断线重连，内置无锁原子化 Session Reaper 自动清理超时僵尸会话。
- **容器与无 Root 环境友好**：内置 `start.sh`，可全自动在 Linux 无 Root 权限环境下便携安装 CMake、Ninja、yt-dlp、QuickJS 并自动编译启动。

### 快速上手

1. **环境准备**：安装 Rust（1.85+，支持 2024 edition）、CMake 和 C++ 编译环境。
2. **克隆与编译运行**：
   ```bash
   git clone https://github.com/alanheng1106/RustLink.git
   cd RustLink
   cargo run --release
   ```
3. **配置**：程序启动时若不存在配置文件，会自动生成 `application.yml`。默认端口为 `2333`，密码为 `youshallnotpass`。支持通过环境变量（如 `PORT`、`LAVALINK_SERVER_PASSWORD`）直接覆盖配置。
4. **测试**：
   ```bash
   cargo test --test smoke_test
   ```

---

## License

This project is licensed under the [MIT License](LICENSE).
