import { useEffect } from "react";
import { useStore } from "./store";
import * as api from "./api";
import Sidebar from "./components/Sidebar";
import MainContent from "./components/MainContent";
import PlayerBar from "./components/PlayerBar";
import LoginModal from "./components/LoginModal";

export default function App() {
  const { session, setSession, dominantColor, setPlayback, setLastfmUser } = useStore();

  // Restore persisted Qobuz + Last.fm sessions on mount
  useEffect(() => {
    api.restoreSession().then(setSession).catch(() => api.getSession().then(setSession).catch(console.error));
    api.lastfmGetSession().then((s) => { if (s) setLastfmUser(s); }).catch(() => {});
  }, []);

  // Poll playback state + auto-advance + scrobbling
  useEffect(() => {
    if (!session.logged_in) return;
    let prevPlaying = false;
    let prevTrackId: string | null = null;
    const scrobbledRef = { scrobbled: false };

    const interval = setInterval(async () => {
      try {
        const state = await api.getPlaybackState();

        // Track just changed → fire "now playing"
        if (state.current_track && state.current_track.id !== prevTrackId) {
          scrobbledRef.scrobbled = false;
          const t = state.current_track;
          api.lastfmNowPlaying(t.title, t.artist, Math.round(t.duration_seconds)).catch(() => {});
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

        // Detect natural track end: was playing → now stopped, same track
        if (prevPlaying && !state.is_playing && state.current_track?.id === prevTrackId) {
          const { repeatMode } = useStore.getState();
          if (repeatMode === 'one' && state.current_track) {
            const src = state.current_track.source;
            if ('Qobuz' in src) {
              await api.playTrack(src.Qobuz.track_id);
              const newState = await api.getPlaybackState();
              setPlayback(newState);
              return;
            }
          } else {
            try {
              const nextState = await api.nextTrack();
              if (nextState) { setPlayback(nextState); return; }
            } catch {}
          }
        }

        prevPlaying = state.is_playing;
        prevTrackId = state.current_track?.id ?? null;
        setPlayback(state);
      } catch {}
    }, 500);
    return () => clearInterval(interval);
  }, [session.logged_in]);

  const [r, g, b] = dominantColor;

  return (
    <div
      className="h-screen w-screen flex flex-col dynamic-bg"
      style={{
        backgroundColor: `rgb(${r}, ${g}, ${b})`,
        backgroundImage: `
          radial-gradient(ellipse at 20% 50%, rgba(${r + 20}, ${g + 20}, ${b + 40}, 0.3) 0%, transparent 50%),
          radial-gradient(ellipse at 80% 20%, rgba(${r + 10}, ${g + 15}, ${b + 30}, 0.2) 0%, transparent 50%)
        `,
      }}
    >
      {/* Login gate */}
      {!session.logged_in && <LoginModal />}

      {/* Main layout */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <Sidebar />

        {/* Content */}
        <MainContent />
      </div>

      {/* Player bar */}
      <PlayerBar />
    </div>
  );
}
