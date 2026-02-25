# Q-Stream — Hi-Res Qobuz Streaming Player

A modern desktop music player for Qobuz Hi-Res streaming, built with **Tauri** (Rust backend) and **React** (TypeScript frontend).

## Features

- 🎵 **Hi-Res Streaming** — FLAC up to 24-bit/192kHz from Qobuz
- 🎼 **Gapless Playback** — Double-buffer pre-queuing for seamless transitions
- 🔀 **Smart Shuffle** — Last.fm-powered recommendations with weighted shuffle algorithm
- 📁 **Local Music** — Import and play your local music library (FLAC, MP3, M4A, etc.)
- 🎨 **Dynamic UI** — Background color adapts to album artwork
- 🖥️ **Glassmorphism Design** — Spotify-inspired interface with blur effects

## Architecture

```
q-stream/
├── src-tauri/               # Rust backend (Tauri)
│   └── src/
│       ├── main.rs          # Entry point & command registration
│       ├── audio.rs         # Hi-Res audio player (rodio/symphonia)
│       ├── qobuz.rs         # Qobuz API client (auth, search, streaming)
│       ├── models.rs        # Data models (tracks, albums, playlists)
│       ├── recommendation.rs # Smart Shuffle engine
│       ├── local_library.rs # Local file scanning & metadata
│       ├── state.rs         # Shared app state
│       └── commands/        # Tauri IPC command handlers
├── src/                     # React frontend
│   ├── App.tsx              # Root component
│   ├── api.ts               # Tauri invoke bridge
│   ├── store.ts             # Zustand state management
│   ├── types.ts             # TypeScript type definitions
│   └── components/          # UI components
│       ├── Sidebar.tsx
│       ├── PlayerBar.tsx
│       ├── MainContent.tsx
│       ├── LoginModal.tsx
│       ├── views/           # Page views
│       └── cards/           # Shared card components
```

## Tech Stack

| Layer        | Technology                         |
|-------------|-------------------------------------|
| Desktop     | Tauri 2                             |
| Backend     | Rust                                |
| Audio       | rodio + symphonia                   |
| Frontend    | React 18 + TypeScript               |
| Styling     | Tailwind CSS                        |
| Animations  | Framer Motion                       |
| State       | Zustand                             |
| API         | Qobuz, Last.fm (recommendations)    |

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v18+)
- [Tauri CLI](https://tauri.app/start/): `cargo install tauri-cli`

## Setup

```bash
# Install frontend dependencies
npm install

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

## Qobuz API

The app handles Qobuz authentication automatically:

1. Extracts `app_id` and secrets from Qobuz web player bundle
2. Logs in with email/password to obtain `user_auth_token`
3. Signs `track/getFileUrl` requests with MD5 hash:
   ```
   md5("trackgetFileUrlformat_id{id}intentstreamtrack_id{id}{timestamp}{secret}")
   ```
4. Prioritizes highest quality: 27 (192kHz) → 7 (96kHz) → 6 (44.1kHz) → 5 (MP3)

## Audio Pipeline

- Download-then-play approach with atomic cache writes
- Double-buffered gapless playback (pre-queues next track)
- Dynamic sample rate switching (recreates audio stream when needed)
- Cubic volume curve for perceptual linearity

## Smart Shuffle

When enabled, the recommendation engine:
1. Queries Last.fm `track.getSimilar` for the current track
2. Cross-references results with the Qobuz catalog
3. Applies weighted shuffle: avoids artist repetition, favors Hi-Res quality
