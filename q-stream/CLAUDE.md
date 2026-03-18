# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Development
npm run tauri dev          # Launch app with hot-reload (starts Vite + Rust in one command)
npm run dev                # Start Vite dev server only (port 5174)

# Build
npm run tauri build        # Production build → src-tauri/target/release/bundle/

# Tests (frontend only — no Rust tests)
npm run test               # Run Vitest suite once
npm run test -- --watch    # Watch mode
npm run test -- --coverage # Coverage report

# Type check
npx tsc --noEmit
```

## Architecture

This is a **Tauri 2 desktop app**: a Rust backend paired with a React/TypeScript frontend. They communicate exclusively through Tauri's IPC (`invoke`/`emit`).

### Data Flow

```
React (Zustand store) ←→ src/api.ts (invoke wrappers) ←→ src-tauri/src/commands/*.rs ←→ core modules
```

- **Frontend → Backend**: `invoke("command_name", { params })` in `api.ts`, handled by `#[tauri::command]` in `commands/`
- **Backend → Frontend**: Tauri events (`track-ended`, `spectrum-data`) emitted by `main.rs`, received via `listen()` in `App.tsx`
- **State**: Zustand in the frontend; `Arc<RwLock<AppState>>` shared across all Rust command handlers (defined in `state.rs`)

### Backend Modules (`src-tauri/src/`)

| Module | Role |
|--------|------|
| `audio.rs` | Core audio engine: rodio + symphonia decoder, IIR biquad EQ, FFT spectrum (2048pt → 80 bins), cpal device management, cubic volume curve |
| `qobuz.rs` | Qobuz API client: auto-extracts app credentials from web player, login, streaming URL resolution (tries 27→7→6→5 quality), download + caching |
| `lastfm.rs` | Last.fm OAuth, now-playing, scrobbling (at 50% play), top-artists fetch |
| `recommendation.rs` | Smart recommendations combining Last.fm top artists, MusicBrainz genres, recent playback, library discovery |
| `local_library.rs` | Recursive folder scan, lofty metadata extraction for FLAC/MP3/M4A/WAV/OGG |
| `persistence.rs` | JSON-on-disk for recently played, dismissed albums, search history |
| `models.rs` | All shared data structures (Qobuz types, playback state, EQ config, etc.) |
| `state.rs` | `AppState` struct with `RwLock`-guarded player, Qobuz client, queue, and mpsc event channel |

### Frontend (`src/`)

- **`App.tsx`**: Root component — initializes sessions from disk, runs polling loop for playback position, handles scrobbling timing, auto-saves state, bridges Tauri events
- **`store.ts`**: Single Zustand store — session, navigation history, playback state, queue, EQ bands/presets, Last.fm session, audio device
- **`api.ts`**: Typed wrappers for every `invoke` call — the only place IPC calls are made
- **`types.ts`**: All TypeScript interfaces mirroring Rust models
- **`components/views/`**: One file per main view (Home, Search, Album, Artist, Playlist, Favorites, Local, Queue, EQ)
- **`tests/player.test.ts`**: Vitest unit tests for pure playback math (formatTime, volume curve, seek/clamp, position tracking)

### Key Architectural Details

- **Audio quality hierarchy**: Format IDs 27 (24-bit/192kHz FLAC) → 7 (24-bit/96kHz) → 6 (16-bit/44.1kHz) → 5 (320kbps MP3). Qobuz client tries each in order until one succeeds.
- **Track caching**: Downloaded audio bytes are cached as `Arc<Vec<u8>>` in `AppState` to avoid re-fetching on replay.
- **EQ**: 5-band (standard) and 10-band (advanced) modes, both using IIR biquad filters applied in the rodio sink pipeline.
- **Spectrum**: Real-time FFT runs in the audio thread; results are emitted as `spectrum-data` events to the frontend ~every frame.
- **Polling vs events**: Playback position is polled by `App.tsx` (interval-based `getPlaybackState` invoke) rather than streamed as events.
- **Navigation**: Custom stack-based history in Zustand (not react-router) — `navigateTo`, `navigateBack` push/pop view state objects.
