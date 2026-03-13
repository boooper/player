# Naviarr

A cross-platform desktop music player for Subsonic and Jellyfin servers, built with Tauri and SvelteKit.

![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-blue)
![Bun](https://img.shields.io/badge/bun-1.0+-black)

---

## Features

- **Multi-server support** — Connect to any Subsonic-compatible or Jellyfin server via saved profiles
- **Last.fm integration** — Scrobbling, now playing updates, and artist taste import
- **Smart recommendations** — Last.fm-powered track and artist recommendations weighted by your liked artists and genres
- **Synced lyrics** — Time-synced and plain lyrics via LRCLib
- **Discord Rich Presence** — Shows your currently playing track in Discord
- **Full library browsing** — Songs, albums, artists, playlists, and starred tracks
- **Queue management** — Shuffle, smart shuffle, repeat, and up-next recommendations
- **Autostart** — Optional launch on login

## Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | [Tauri v2](https://tauri.app) (Rust) |
| Frontend | [SvelteKit](https://kit.svelte.dev) + [Tailwind CSS v4](https://tailwindcss.com) |
| UI components | [shadcn-svelte](https://shadcn-svelte.com) |
| Backend/DB | Rust (SQLite via the Tauri process) |
| Package manager | [Bun](https://bun.sh) |

## Project Structure

```
apps/
  web/        # SvelteKit frontend (adapter-static)
  desktop/    # Tauri shell
packages/
  shared/     # Shared TypeScript types and contracts
```

## Getting Started

### Prerequisites

- [Bun](https://bun.sh) 1.0+
- [Rust](https://rustup.rs) (stable)
- Platform dependencies for Tauri — see the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/)

### Install


### Development

Run the full desktop app (starts the web dev server then launches Tauri):

```bash
bun run dev:desktop
```

### Build

```bash
bun run build:desktop
```

## Configuration

On first launch, add a server profile in **Settings → Profiles** with your Subsonic or Jellyfin server URL and credentials.

Optionally add a [Last.fm API key](https://www.last.fm/api/account/create) in **Settings → Last.fm** to enable scrobbling and recommendations.