# AQiu

A modern, beautiful proxy client for Mihomo with an iOS-inspired "AQiu" design.

## Features

- 🎨 **AQiu UI** - iOS-style glassmorphism design with blur effects
- ⚡ **Real-time Dashboard** - Live traffic monitoring and connection stats
- 🌐 **Proxy Management** - Visual proxy group switching and latency testing
- 📜 **Rules Viewer** - Search and browse routing rules
- 🔌 **Connections Monitor** - Real-time active connections list
- 📋 **Live Logs** - Streaming log viewer with filtering
- ⚙️ **Settings** - System proxy control and TUN mode support

## Requirements

- [Bun](https://bun.sh/) - JavaScript runtime
- [Rust](https://www.rust-lang.org/) - For Tauri backend
- [Mihomo](https://github.com/MetaCubeX/mihomo) - Core proxy engine

## Quick Start

### 1. Install Dependencies

```bash
bun install
```

### 2. Download Mihomo Core

Download the Mihomo binary for your platform from [releases](https://github.com/MetaCubeX/mihomo/releases) and place it at:

- **Windows**: `%LOCALAPPDATA%\aqiu\mihomo\mihomo.exe`
- **macOS**: `~/Library/Application Support/aqiu/mihomo/mihomo`
- **Linux**: `~/.local/share/aqiu/mihomo/mihomo`

### 3. Create Config

Copy the default config to your config directory:

- **Windows**: `%LOCALAPPDATA%\aqiu\config\config.yaml`
- **macOS**: `~/Library/Application Support/aqiu/config/config.yaml`
- **Linux**: `~/.local/share/aqiu/config/config.yaml`

### 4. Run Development Server

```bash
bun run tauri dev
```

### 5. Build for Production

```bash
bun run tauri build
```

## Project Structure

```
aqiu/
├── src/                    # Vue.js frontend
│   ├── api/               # API clients
│   │   ├── mihomo.ts      # Mihomo REST API
│   │   └── tauri.ts       # Tauri commands
│   ├── composables/       # Vue composables
│   │   ├── useMihomo.ts   # Mihomo state management
│   │   └── useCore.ts     # Core process management
│   ├── assets/            # Static assets
│   │   └── style.css      # AQiu design system
│   ├── App.vue            # Main application
│   └── main.ts            # Entry point
├── src-tauri/             # Rust backend
│   └── src/
│       ├── lib.rs         # Main entry
│       └── core.rs        # Core management
├── resources/             # Bundled resources
│   └── config.yaml       # Default config
└── prototypes/           # Design prototypes
```

## API Integration

The app connects to Mihomo's REST API at `http://127.0.0.1:29090` (configurable).

### Supported Endpoints

- `GET /configs` - Configuration
- `GET /proxies` - Proxy list and groups
- `PUT /proxies/:group` - Select proxy
- `GET /proxies/:proxy/delay` - Test latency
- `GET /rules` - Rule list
- `GET /connections` - Active connections
- `WS /traffic` - Real-time traffic
- `WS /logs` - Real-time logs

## Tech Stack

- **Frontend**: Vue 3 + TypeScript + Vite
- **Backend**: Tauri 2 + Rust
- **Styling**: Custom CSS (AQiu design system)
- **Icons**: Font Awesome 6

## Verification

- Run `bun run tauri dev`
- Start the core from the UI and confirm the dashboard updates
- Toggle system proxy on/off and verify it is applied, then quit the app to ensure it is cleared

## License

MIT
