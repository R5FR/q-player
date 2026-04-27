import { useEffect } from "react";
import { AnimatePresence } from "framer-motion";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "./store";
import * as api from "./api";
import Sidebar from "./components/Sidebar";
import MainContent from "./components/MainContent";
import PlayerBar from "./components/PlayerBar";
import LoginModal from "./components/LoginModal";
import FullscreenPlayer from "./components/FullscreenPlayer";

export default function App() {
  const {
    session, setSession, dominantColor, setPlayback, setLastfmUser, addRecentlyPlayed,
    recentlyPlayed, dismissedAlbums, searchHistory,
    setRecentlyPlayed, setDismissedAlbums, setSearchHistory,
    isSeeking, sleepTimerEndMs, setSleepTimer,
    isDarkMode,
    isFullscreen, setIsFullscreen,
    setEqBands, setEqBandsAdvanced, setEqEnabled, setEqAdvanced, setSelectedDevice,
    setLocalTracks, setMusicFolder,
  } = useStore();

  // Apply dark/light class on <html> for CSS variable theming
  useEffect(() => {
    document.documentElement.classList.toggle("light", !isDarkMode);
  }, [isDarkMode]);

  // Restore persisted Qobuz + Last.fm sessions, then load app data and user config from disk
  useEffect(() => {
    api.restoreSession().then(setSession).catch(() => api.getSession().then(setSession).catch(console.error));
    api.lastfmGetSession().then((s) => { if (s) setLastfmUser(s); }).catch(() => {});

    api.loadAppData().then((data) => {
      if (data.recently_played.length) setRecentlyPlayed(data.recently_played);
      if (data.dismissed_albums.length) setDismissedAlbums(data.dismissed_albums);
      if (data.search_history.length) setSearchHistory(data.search_history);
    }).catch(() => {});

    // Restore user config: EQ bands, EQ mode, and selected audio device.
    // Volume is already applied to the audio engine by the backend at startup.
    api.loadConfig().then((cfg) => {
      // Default band definitions (freq/q/label), same as store.ts
      const defaults5 = [
        { freq: 60,    q: 0.9, label: "Bass"     },
        { freq: 250,   q: 1.0, label: "Low Mid"  },
        { freq: 1000,  q: 1.0, label: "Mid"      },
        { freq: 4000,  q: 1.0, label: "High Mid" },
        { freq: 14000, q: 0.9, label: "Treble"   },
      ];
      const defaults10 = [
        { freq: 32,    q: 0.9, label: "Sub"      },
        { freq: 64,    q: 0.9, label: "Bass"     },
        { freq: 125,   q: 1.0, label: "Low Bass" },
        { freq: 250,   q: 1.0, label: "Low Mid"  },
        { freq: 500,   q: 1.0, label: "Mid"      },
        { freq: 1000,  q: 1.0, label: "Upper"    },
        { freq: 2000,  q: 1.0, label: "Presence" },
        { freq: 4000,  q: 1.0, label: "High Mid" },
        { freq: 8000,  q: 0.9, label: "High"     },
        { freq: 16000, q: 0.9, label: "Air"      },
      ];
      if (cfg.eq_advanced && cfg.eq_bands.length === 10) {
        setEqBandsAdvanced(defaults10.map((d, i) => ({ ...d, gain: cfg.eq_bands[i]?.gain_db ?? 0 })));
      } else if (!cfg.eq_advanced && cfg.eq_bands.length === 5) {
        setEqBands(defaults5.map((d, i) => ({ ...d, gain: cfg.eq_bands[i]?.gain_db ?? 0 })));
      }
      setEqEnabled(cfg.eq_enabled);
      setEqAdvanced(cfg.eq_advanced);
      if (cfg.audio_device && cfg.audio_device !== "Default") {
        setSelectedDevice(cfg.audio_device);
      }
    }).catch(() => {});

    // Resolve effective music folder then auto-scan it
    api.getDefaultMusicFolder().then((folder) => {
      setMusicFolder(folder);
      return api.scanMusicFolder();
    }).then(setLocalTracks).catch(() => {});
  }, []);

  // Auto-save persistent data 1 s after any change (debounced)
  useEffect(() => {
    const timer = setTimeout(() => {
      api.saveAppData({
        recently_played: recentlyPlayed,
        dismissed_albums: dismissedAlbums,
        search_history: searchHistory,
      }).catch(() => {});
    }, 1000);
    return () => clearTimeout(timer);
  }, [recentlyPlayed, dismissedAlbums, searchHistory]);

  // Sleep timer — stop playback when timer expires
  useEffect(() => {
    if (!sleepTimerEndMs) return;
    const remaining = sleepTimerEndMs - Date.now();
    if (remaining <= 0) { setSleepTimer(null); return; }
    const t = setTimeout(async () => {
      try { await api.pause(); } catch {}
      setSleepTimer(null);
    }, remaining);
    return () => clearTimeout(t);
  }, [sleepTimerEndMs]);

  // Auto-start Qobuz Connect as soon as the session is available; stop on logout or unmount
  useEffect(() => {
    if (!session.logged_in) return;
    api.startQobuzConnect().catch(() => {});
    api.scanConnectDevices().catch(() => {});
    return () => { api.stopQobuzConnect().catch(() => {}); };
  }, [session.logged_in]);

  // Stop Connect cleanly when the app window closes
  useEffect(() => {
    const onClose = () => { api.stopQobuzConnect().catch(() => {}); };
    window.addEventListener("beforeunload", onClose);
    return () => window.removeEventListener("beforeunload", onClose);
  }, []);

  // Poll playback state + scrobbling + smart queue refill
  useEffect(() => {
    if (!session.logged_in) return;
    let prevTrackId: string | null = null;
    const scrobbledRef = { scrobbled: false };
    // Track ID for which we last auto-enqueued similar tracks (avoid duplicates)
    const lastEnqueuedForRef = { trackId: null as string | null };

    const interval = setInterval(async () => {
      // Don't overwrite position while user is dragging the seek bar
      if (useStore.getState().isSeeking) return;
      try {
        const state = await api.getPlaybackState();

        // Track just changed → fire "now playing" + check if queue needs refilling
        if (state.current_track && state.current_track.id !== prevTrackId) {
          scrobbledRef.scrobbled = false;
          const t = state.current_track;
          addRecentlyPlayed(t);
          api.lastfmNowPlaying(t.title, t.artist, Math.round(t.duration_seconds)).catch(() => {});

          // Auto-fill queue with similar tracks when ≤ 2 remain after current
          if (lastEnqueuedForRef.trackId !== t.id) {
            api.getQueue().then((q) => {
              const remaining = q.tracks.length - ((q.current_index ?? 0) + 1);
              if (remaining <= 2) {
                lastEnqueuedForRef.trackId = t.id;
                api.enqueueSimilar(t.title, t.artist).catch(() => {});
              }
            }).catch(() => {});
          }
        }

        // Scrobble when 50% of track played (and not yet scrobbled this track)
        if (
          !scrobbledRef.scrobbled &&
          state.current_track &&
          state.duration_ms > 0 &&
          state.position_ms >= Math.min(state.duration_ms * 0.5, 4 * 60 * 1000)
        ) {
          scrobbledRef.scrobbled = true;
          const t = state.current_track;
          api.lastfmScrobble(t.title, t.artist, Math.round(t.duration_seconds)).catch(() => {});
        }

        prevTrackId = state.current_track?.id ?? null;
        setPlayback(state);
      } catch {}
    }, 500);
    return () => clearInterval(interval);
  }, [session.logged_in]);

  // Listen for track-ended event from backend (instant, no polling delay)
  useEffect(() => {
    if (!session.logged_in) return;

    const unlisten = listen("track-ended", async () => {
      const { repeatMode, playback: currentPlayback } = useStore.getState();
      const track = currentPlayback.current_track;

      if (repeatMode === "one" && track) {
        const src = track.source;
        if ("Qobuz" in src) {
          try {
            const newState = await api.playTrack(src.Qobuz.track_id);
            setPlayback(newState);
          } catch (e) {
            console.error("Repeat-one failed:", e);
          }
          return;
        }
      }

      try {
        const nextState = await api.nextTrack();
        if (nextState) {
          setPlayback(nextState);
        }
      } catch (e) {
        console.error("Auto-advance failed:", e);
      }
    });

    return () => { unlisten.then((fn) => fn()); };
  }, [session.logged_in]);

  const [r, g, b] = dominantColor;

  // Dark: crush album colour to near-black so it barely tints the void background
  // Light: lift album colour toward warm parchment (#f5f0e8 = 245 240 232)
  const bgColor = isDarkMode
    ? `rgb(${Math.floor(r * 0.06) + 4}, ${Math.floor(g * 0.05) + 4}, ${Math.floor(b * 0.10) + 10})`
    : `rgb(${Math.min(255, Math.floor(245 - (255 - r) * 0.06))}, ${Math.min(255, Math.floor(240 - (255 - g) * 0.05))}, ${Math.min(255, Math.floor(232 - (255 - b) * 0.04))})`;

  const bgImage = isDarkMode
    ? `
        radial-gradient(ellipse at 15% 60%, rgb(var(--qs-accent) / 0.04) 0%, transparent 55%),
        radial-gradient(ellipse at 85% 15%, rgb(var(--qs-accent-2) / 0.04) 0%, transparent 55%),
        radial-gradient(ellipse at 50% 90%, rgba(${Math.floor(r * 0.4)}, ${Math.floor(g * 0.35)}, ${Math.floor(b * 0.5)}, 0.05) 0%, transparent 50%)
      `
    : `
        radial-gradient(ellipse at 20% 45%, rgba(${Math.floor(r * 0.3)}, ${Math.floor(g * 0.28)}, ${Math.floor(b * 0.22)}, 0.06) 0%, transparent 55%),
        radial-gradient(ellipse at 78% 20%, rgb(var(--qs-accent) / 0.04) 0%, transparent 45%),
        radial-gradient(ellipse at 50% 95%, rgba(${Math.floor(r * 0.2)}, ${Math.floor(g * 0.18)}, ${Math.floor(b * 0.15)}, 0.04) 0%, transparent 40%)
      `;

  return (
    <>
      {/* ── Fullscreen player — rendered at root level, above everything ── */}
      <AnimatePresence>
        {isFullscreen && (
          <FullscreenPlayer onClose={() => setIsFullscreen(false)} />
        )}
      </AnimatePresence>

      {/* ── Normal app layout ── */}
      <div
        className="h-screen w-screen flex flex-col dynamic-bg"
        style={{ backgroundColor: bgColor, backgroundImage: bgImage }}
      >
        {!session.logged_in ? (
          /* Login gate — don't render main content until authenticated */
          <LoginModal />
        ) : (
          <>
            {/* Main layout */}
            <div className="flex flex-1 overflow-hidden">
              <Sidebar />
              <MainContent />
            </div>
            {/* Player bar */}
            <PlayerBar />
          </>
        )}
      </div>
    </>
  );
}
