import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "./store";
import * as api from "./api";
import Sidebar from "./components/Sidebar";
import MainContent from "./components/MainContent";
import PlayerBar from "./components/PlayerBar";
import LoginModal from "./components/LoginModal";

export default function App() {
  const {
    session, setSession, dominantColor, setPlayback, setLastfmUser, addRecentlyPlayed,
    recentlyPlayed, dismissedAlbums, searchHistory,
    setRecentlyPlayed, setDismissedAlbums, setSearchHistory,
    isSeeking, sleepTimerEndMs, setSleepTimer,
    isDarkMode,
  } = useStore();

  // Apply dark/light class on <html> for CSS variable theming
  useEffect(() => {
    document.documentElement.classList.toggle("light", !isDarkMode);
  }, [isDarkMode]);

  // Restore persisted Qobuz + Last.fm sessions, then load app data from disk
  useEffect(() => {
    api.restoreSession().then(setSession).catch(() => api.getSession().then(setSession).catch(console.error));
    api.lastfmGetSession().then((s) => { if (s) setLastfmUser(s); }).catch(() => {});

    api.loadAppData().then((data) => {
      if (data.recently_played.length) setRecentlyPlayed(data.recently_played);
      if (data.dismissed_albums.length) setDismissedAlbums(data.dismissed_albums);
      if (data.search_history.length) setSearchHistory(data.search_history);
    }).catch(() => {});
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
  // Crush album colour to near-black so it barely tints the dark background
  const dr = Math.floor(r * 0.06) + 4;
  const dg = Math.floor(g * 0.05) + 4;
  const db = Math.floor(b * 0.10) + 10;

  return (
    <div
      className="h-screen w-screen flex flex-col dynamic-bg"
      style={{
        backgroundColor: `rgb(${dr}, ${dg}, ${db})`,
        backgroundImage: `
          radial-gradient(ellipse at 15% 60%, rgb(var(--qs-accent) / 0.04) 0%, transparent 55%),
          radial-gradient(ellipse at 85% 15%, rgb(var(--qs-accent-2) / 0.04) 0%, transparent 55%),
          radial-gradient(ellipse at 50% 90%, rgba(${Math.floor(r * 0.4)}, ${Math.floor(g * 0.35)}, ${Math.floor(b * 0.5)}, 0.05) 0%, transparent 50%)
        `,
      }}
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
  );
}
