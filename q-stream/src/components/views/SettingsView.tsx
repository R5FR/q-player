import { useState } from "react";
import { motion } from "framer-motion";
import { Settings, FolderOpen, Check } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { useStore } from "../../store";
import * as api from "../../api";

export default function SettingsView() {
  const { musicFolder, setMusicFolder, setLocalTracks } = useStore();
  const [scanning, setScanning] = useState(false);
  const [saved, setSaved] = useState(false);

  const handleBrowse = async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected) return;
      const folder = selected as string;
      setScanning(true);
      const tracks = await api.setMusicFolder(folder);
      setMusicFolder(folder);
      setLocalTracks(tracks);
      setScanning(false);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error(e);
      setScanning(false);
    }
  };

  const handleManualSave = async (value: string) => {
    if (!value.trim()) return;
    try {
      setScanning(true);
      const tracks = await api.setMusicFolder(value.trim());
      setMusicFolder(value.trim());
      setLocalTracks(tracks);
      setScanning(false);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error(e);
      setScanning(false);
    }
  };

  return (
    <div className="p-8 max-w-2xl">
      <div className="flex items-center gap-3 mb-8">
        <Settings className="w-6 h-6 text-qs-accent" />
        <h1 className="text-2xl font-bold text-white">Paramètres</h1>
      </div>

      <section className="space-y-4">
        <div>
          <h2 className="text-xs font-condensed font-semibold text-qs-text-dim uppercase tracking-[0.15em] mb-4">
            Bibliothèque locale
          </h2>

          <div className="glass rounded-xl p-4 space-y-3">
            <p className="text-sm text-qs-text-dim">
              Dossier contenant vos fichiers audio locaux. Q-Stream le scanne
              automatiquement au démarrage.
            </p>

            <div className="flex items-center gap-2">
              <input
                type="text"
                value={musicFolder}
                onChange={(e) => setMusicFolder(e.target.value)}
                onBlur={(e) => handleManualSave(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleManualSave((e.target as HTMLInputElement).value);
                }}
                placeholder="Chemin du dossier musical..."
                className="input-cyber flex-1 text-sm py-2 px-3"
              />
              <motion.button
                whileTap={{ scale: 0.95 }}
                onClick={handleBrowse}
                disabled={scanning}
                className="flex items-center gap-2 px-4 py-2 glass rounded-xl text-sm font-medium hover:bg-white/10 transition disabled:opacity-50 flex-shrink-0"
              >
                <FolderOpen className="w-4 h-4" />
                Parcourir
              </motion.button>
            </div>

            <div className="h-5 flex items-center">
              {scanning && (
                <p className="text-xs text-qs-text-dim animate-pulse">Scan en cours…</p>
              )}
              {saved && !scanning && (
                <p className="text-xs text-qs-green flex items-center gap-1">
                  <Check className="w-3 h-3" /> Dossier enregistré et scanné
                </p>
              )}
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
