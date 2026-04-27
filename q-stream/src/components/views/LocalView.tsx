import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { FolderOpen, Play, Music, Settings, RefreshCw } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";

export default function LocalView() {
  const { localTracks, setLocalTracks, musicFolder, setView } = useStore();
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    api.getLocalTracks().then(setLocalTracks).catch(console.error);
  }, []);

  const handleRefresh = async () => {
    try {
      setLoading(true);
      const tracks = await api.scanMusicFolder();
      setLocalTracks(tracks);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const playTrack = async (filePath: string) => {
    try {
      await api.playLocalTrack(filePath);
    } catch (e) {
      console.error(e);
    }
  };

  const formatTime = (seconds: number) => {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  };

  const albums = localTracks.reduce(
    (acc, track) => {
      const key = `${track.artist} - ${track.album}`;
      if (!acc[key]) acc[key] = [];
      acc[key].push(track);
      return acc;
    },
    {} as Record<string, typeof localTracks>
  );

  return (
    <div className="p-8">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-3">
          <FolderOpen className="w-6 h-6 text-qs-green" />
          <h1 className="text-2xl font-bold text-white">Musique locale</h1>
          <span className="text-sm text-qs-text-dim">
            {localTracks.length} titres
          </span>
        </div>
        <div className="flex items-center gap-2">
          <motion.button
            whileTap={{ scale: 0.95 }}
            onClick={handleRefresh}
            disabled={loading}
            title="Rescanner le dossier"
            className="flex items-center gap-2 px-3 py-2 glass rounded-xl text-sm font-medium hover:bg-white/10 transition disabled:opacity-50"
          >
            <RefreshCw className={`w-4 h-4 ${loading ? "animate-spin" : ""}`} />
          </motion.button>
          <motion.button
            whileTap={{ scale: 0.95 }}
            onClick={() => setView("settings")}
            title="Paramètres"
            className="flex items-center gap-2 px-3 py-2 glass rounded-xl text-sm font-medium hover:bg-white/10 transition"
          >
            <Settings className="w-4 h-4" />
          </motion.button>
        </div>
      </div>

      {musicFolder && (
        <p className="text-xs text-qs-text-dim mb-6 truncate" title={musicFolder}>
          {musicFolder}
        </p>
      )}

      {localTracks.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-qs-text-dim">
          <Music className="w-12 h-12 mb-4 opacity-30" />
          <p className="text-lg">Aucune musique locale trouvée</p>
          <p className="text-sm mt-1">
            Vérifiez le dossier dans{" "}
            <button
              onClick={() => setView("settings")}
              className="text-qs-accent underline hover:text-qs-accent/80"
            >
              Paramètres
            </button>
          </p>
          <p className="text-xs mt-3 text-qs-text-dim">
            Formats supportés : FLAC, MP3, M4A, AAC, OGG, WAV, AIFF
          </p>
        </div>
      ) : (
        <div className="space-y-8">
          {Object.entries(albums).map(([albumKey, tracks]) => (
            <div key={albumKey}>
              <div className="flex items-center gap-3 mb-3">
                <div className="w-12 h-12 rounded-lg overflow-hidden bg-qs-surface flex-shrink-0">
                  {tracks[0].cover_data ? (
                    <img
                      src={tracks[0].cover_data}
                      alt=""
                      className="w-full h-full object-cover"
                    />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center text-xl">
                      💿
                    </div>
                  )}
                </div>
                <div>
                  <p className="text-sm font-semibold text-white">
                    {tracks[0].album}
                  </p>
                  <p className="text-xs text-qs-text-dim">{tracks[0].artist}</p>
                </div>
              </div>

              <div className="space-y-0.5 ml-4">
                {tracks
                  .sort((a, b) => (a.track_number || 0) - (b.track_number || 0))
                  .map((track) => (
                    <motion.button
                      key={track.file_path}
                      whileHover={{ x: 2 }}
                      onClick={() => playTrack(track.file_path)}
                      className="w-full flex items-center gap-4 px-4 py-2.5 rounded-lg hover:bg-white/5 transition group text-left"
                    >
                      <span className="w-6 text-xs text-qs-text-dim text-center group-hover:hidden">
                        {track.track_number || "-"}
                      </span>
                      <Play className="w-3.5 h-3.5 text-white hidden group-hover:block w-6 text-center" />

                      <span className="flex-1 text-sm text-white truncate">
                        {track.title}
                      </span>

                      <span className="quality-local text-[9px]">
                        {track.format}
                        {track.bit_depth && track.sample_rate
                          ? ` ${track.bit_depth}b/${(track.sample_rate / 1000).toFixed(1)}kHz`
                          : ""}
                      </span>

                      <span className="text-xs text-qs-text-dim w-10 text-right">
                        {formatTime(track.duration_seconds)}
                      </span>
                    </motion.button>
                  ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
