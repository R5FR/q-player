# Q-Stream — Hi-Res Qobuz Streaming Player

A modern desktop music player for **Qobuz Hi-Res** streaming (up to 24-bit / 192 kHz), built with **Tauri 2** (Rust) and **React 18** (TypeScript).

---

## Features

- 🎵 **Hi-Res Streaming** — FLAC up to 24-bit/192 kHz directly from Qobuz
- 🔊 **Seek & drag** — Click or drag the progress bar to jump anywhere in a track
- 💿 **Album playback** — Click any track in an album context to queue the whole album and start from that position
- 🔀 **Smart Shuffle** — Last.fm-powered smart shuffle with weighted artist-deduplication
- 🤖 **Auto-Radio** — When your queue runs low, similar tracks are automatically enqueued via Last.fm
- 🏠 **Personalised Home** — "Your Artists on Qobuz" section based on your Last.fm listening history (top artists, last 3 months)
- 📁 **Local Music** — Import and play local files (FLAC, MP3, M4A, WAV, OGG…)
- 🎨 **Dynamic UI** — Background colour adapts to the current album artwork
- 🎙️ **Last.fm Scrobbling** — Automatic now-playing updates and scrobbling at 50 % playtime
- ↩️ **Shuffle / Repeat** — Repeat-off / Repeat-all / Repeat-one, with queue persistence

---

## Prerequisites

### Windows (primary target)

| Tool | Version | Install |
|------|---------|---------|
| **Rust** (stable) | ≥ 1.77 | [rustup.rs](https://rustup.rs/) |
| **Node.js** | ≥ 18 LTS | [nodejs.org](https://nodejs.org/) |
| **WebView2** | any | bundled with Windows 11; [download](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) for Windows 10 |
| **Visual Studio Build Tools** | 2019 or 2022 | needed by Rust on Windows — select "Desktop development with C++" |
| **Tauri CLI** | v2 | `cargo install tauri-cli --version "^2"` |

### Linux

```bash
# Debian / Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev
```

### macOS

```bash
xcode-select --install
```

---

## Installation

```bash
# 1. Clone the repo
git clone https://github.com/yourname/q-stream.git
cd q-stream

# 2. Install frontend dependencies
npm install
```

---

## Running in Development Mode

```bash
npm run tauri dev
```

The first run compiles all Rust crates (~1–3 min). Subsequent runs use the incremental cache.

Hot-reload is active for the React frontend; Rust changes require a full re-compile.

---

## Building for Production

```bash
npm run tauri build
```

The installer / executable is written to `src-tauri/target/release/bundle/`.

| Platform | Output |
|----------|--------|
| Windows | `q-stream_x.x.x_x64-setup.exe` (NSIS) and `q-stream_x.x.x_x64.msi` |
| Linux | `.deb` and `.AppImage` |
| macOS | `.dmg` |

---

## First Launch

1. Open Q-Stream.
2. Click **Sign in** and enter your **Qobuz** email and password.  
   The app extracts the `app_id` and secrets from the Qobuz web player automatically — no manual configuration needed.
3. *(Optional)* Connect **Last.fm** via **Settings → Last.fm** to enable:
   - Scrobbling
   - Auto-Radio (queue auto-fill with similar tracks)
   - Personalised home page ("Your Artists on Qobuz")

---

## Last.fm Setup

1. Open the app → top-right avatar → **Connect Last.fm**
2. The app opens your browser at `last.fm/api/auth/…`
3. Authorise Q-Stream on the Last.fm page
4. Return to the app — authenticate confirmaton completes automatically
5. Your Last.fm username now appears in the header

Once connected, the **Home** page adds a "Your Artists on Qobuz" section based on your top-listened artists over the past 3 months.

---

## Audio Quality

Q-Stream always tries the highest quality available for each track, in order:

| Format ID | Quality |
|-----------|---------|
| 27 | 24-bit / 192 kHz FLAC |
| 7 | 24-bit / 96 kHz FLAC |
| 6 | 16-bit / 44.1 kHz FLAC (CD) |
| 5 | 320 kbps MP3 (fallback) |

The quality badge in the player bar shows exactly what is being streamed.  
Tracks are cached to disk after the first play for instant replay.

---

## Architecture

```
q-stream/
├── src-tauri/                 # Rust backend (Tauri 2)
│   └── src/
│       ├── main.rs            # Entry point & Tauri command registration
│       ├── audio.rs           # Hi-Res audio engine (rodio + symphonia)
│       ├── qobuz.rs           # Qobuz API client (auth, search, streaming, cache)
│       ├── lastfm.rs          # Last.fm client (auth, scrobbling)
│       ├── recommendation.rs  # Smart Shuffle + personalised recommendations
│       ├── local_library.rs   # Local file scanning & metadata extraction
│       ├── models.rs          # Shared data models
│       ├── state.rs           # Shared app state (RwLock)
│       └── commands/          # Tauri IPC command handlers
│           ├── auth.rs
│           ├── browse.rs
│           ├── favorites.rs
│           ├── lastfm.rs
│           ├── local_library.rs
│           ├── playback.rs    # play_track, seek, next/prev, play_from_queue
│           ├── queue.rs       # add_to_queue, smart_shuffle, enqueue_similar
│           ├── recommendations.rs
│           └── ui.rs
└── src/                       # React 18 + TypeScript frontend
    ├── App.tsx                # Root component + polling loop
    ├── api.ts                 # Tauri invoke wrappers
    ├── store.ts               # Zustand global state
    ├── types.ts               # TypeScript type definitions
    └── components/
        ├── PlayerBar.tsx      # Playback controls + seek bar
        ├── Sidebar.tsx
        ├── MainContent.tsx
        ├── views/
        │   ├── HomeView.tsx   # Personalised home
        │   ├── AlbumView.tsx  # Album + track list
        │   ├── FavoritesView.tsx
        │   ├── SearchView.tsx
        │   └── …
        └── cards/
            ├── TrackRow.tsx   # Reusable track row (album-aware)
            ├── AlbumCard.tsx
            └── …
```

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop runtime | Tauri 2 |
| Backend language | Rust (stable) |
| Audio decoding | rodio 0.19 + symphonia-all |
| HTTP client | reqwest 0.12 |
| Frontend | React 18 + TypeScript (Vite) |
| Styling | Tailwind CSS |
| Animations | Framer Motion |
| State | Zustand |
| Recommendations | Last.fm public API |

---

## Troubleshooting

**App won't start / blank window**  
→ Ensure WebView2 is installed (Windows 10).

**"Not logged in" error on playback**  
→ Re-login: Qobuz sessions expire after ~30 days.

**Audio cuts out / choppy on Hi-Res tracks**  
→ Check that your audio device supports the sample rate (WASAPI on Windows, PipeWire on Linux).  
→ The app will fall back to a lower quality tier automatically if the top tier fails.

**Last.fm personalised section is empty**  
→ You need at least a few weeks of scrobble history. Section auto-populates once data is available.

**Slow "Play All" / loading spinner**  
→ Normal — the first play of any track downloads the FLAC file (10–100 MB). Subsequent plays are instant (cache hit).

