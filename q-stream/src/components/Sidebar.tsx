import { useState } from "react";
import { motion } from "framer-motion";
import {
  Home,
  Search,
  ListMusic,
  FolderOpen,
  Disc3,
  LogOut,
  User,
  Sparkles,
  Radio,
  CheckCircle,
  Loader2,
  Library,
  BookMarked,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-shell";
import { useStore } from "../store";
import * as api from "../api";
import type { ViewType } from "../types";

const TOP_NAV: Array<{ icon: typeof Home; label: string; view: ViewType }> = [
  { icon: Home, label: "Accueil", view: "home" },
  { icon: Search, label: "Rechercher", view: "search" },
];

const LIBRARY_NAV: Array<{
  icon: typeof Home;
  label: string;
  view: ViewType;
  suffix?: React.ReactNode;
}> = [
  { icon: BookMarked, label: "Bibliothèque", view: "favorites" },
  {
    icon: ListMusic,
    label: "File d'attente",
    view: "queue",
    suffix: <Sparkles className="w-3 h-3 text-qs-accent" />,
  },
  { icon: FolderOpen, label: "Musique locale", view: "local" },
];

export default function Sidebar() {
  const { currentView, setView, session, setSession, lastfmUser, setLastfmUser } = useStore();
  const [lfmState, setLfmState] = useState<"idle" | "pending" | "loading">("idle");

  const handleLogout = async () => {
    await api.logout();
    setSession({ logged_in: false });
  };

  const handleLastfmConnect = async () => {
    try {
      setLfmState("loading");
      const url = await api.lastfmStartAuth();
      await open(url);
      setLfmState("pending");
    } catch (e) {
      console.error(e);
      setLfmState("idle");
    }
  };

  const handleLastfmDone = async () => {
    try {
      setLfmState("loading");
      const session = await api.lastfmCompleteAuth();
      setLastfmUser(session);
      setLfmState("idle");
    } catch (e) {
      console.error("Last.fm auth failed:", e);
      setLfmState("idle");
    }
  };

  const handleLastfmDisconnect = async () => {
    await api.lastfmDisconnect();
    setLastfmUser(null);
  };

  const NavBtn = ({
    icon: Icon,
    label,
    view,
    suffix,
  }: {
    icon: typeof Home;
    label: string;
    view: ViewType;
    suffix?: React.ReactNode;
  }) => (
    <motion.button
      whileTap={{ scale: 0.98 }}
      onClick={() => setView(view)}
      className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium ${
        currentView === view ? "nav-item-active" : "nav-item-inactive"
      }`}
    >
      <Icon className="w-4 h-4 flex-shrink-0" />
      <span className="truncate">{label}</span>
      {suffix && <span className="ml-auto">{suffix}</span>}
    </motion.button>
  );

  return (
    <aside className="w-64 h-full flex flex-col gap-2 p-2 flex-shrink-0">
      {/* ── Panel 1 : navigation principale ── */}
      <div className="glass-light rounded-xl p-3 space-y-1">
        {/* Brand */}
        <div className="flex items-center gap-3 px-3 py-2 mb-1">
          <motion.div
            whileHover={{ rotate: 180 }}
            transition={{ duration: 0.6 }}
            className="w-8 h-8 rounded-lg bg-gradient-to-br from-qs-accent to-qs-accent-2 flex items-center justify-center flex-shrink-0"
          >
            <Disc3 className="w-4 h-4 text-white" />
          </motion.div>
          <div className="min-w-0">
            <h1 className="text-sm font-bold text-white leading-none">Q-Stream</h1>
            <p className="text-[9px] text-qs-accent font-medium tracking-widest uppercase mt-0.5">
              Hi-Res Audio
            </p>
          </div>
        </div>

        {TOP_NAV.map((item) => (
          <NavBtn key={item.view} {...item} />
        ))}
      </div>

      {/* ── Panel 2 : bibliothèque ── */}
      <div className="glass-light rounded-xl flex-1 flex flex-col overflow-hidden min-h-0">
        {/* En-tête */}
        <div className="flex items-center gap-2 px-4 pt-4 pb-2 flex-shrink-0">
          <Library className="w-4 h-4 text-qs-text-dim" />
          <span className="text-sm font-semibold text-qs-text">Votre bibliothèque</span>
        </div>

        {/* Items */}
        <div className="flex-1 overflow-y-auto px-2 py-1 space-y-0.5 min-h-0">
          {LIBRARY_NAV.map((item) => (
            <NavBtn key={item.view} {...item} />
          ))}
        </div>

        {/* ── Utilisateur ── */}
        {session.logged_in && (
          <div className="p-3 border-t border-white/5 space-y-2 flex-shrink-0">
            {/* Qobuz */}
            <div className="flex items-center gap-2.5 px-1">
              <div className="w-7 h-7 rounded-full bg-qs-accent/15 flex items-center justify-center flex-shrink-0">
                <User className="w-3.5 h-3.5 text-qs-accent" />
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-xs font-medium text-white truncate">{session.user_name}</p>
                {session.subscription && (
                  <p className="text-[9px] text-qs-accent">{session.subscription}</p>
                )}
              </div>
            </div>

            {/* Last.fm */}
            {lastfmUser ? (
              <div className="flex items-center gap-2 px-1">
                <Radio className="w-3 h-3 text-red-400 flex-shrink-0" />
                <span className="text-xs text-white/60 truncate flex-1">{lastfmUser.user_name}</span>
                <button
                  onClick={handleLastfmDisconnect}
                  className="text-[10px] text-qs-text-dim hover:text-red-400 transition"
                  title="Déconnecter Last.fm"
                >
                  ✕
                </button>
              </div>
            ) : lfmState === "pending" ? (
              <div className="space-y-1.5 px-1">
                <p className="text-[10px] text-qs-text-dim leading-tight">
                  Autorise Q-Stream dans ton navigateur, puis clique sur Fait.
                </p>
                <button
                  onClick={handleLastfmDone}
                  className="w-full flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-lg text-xs bg-red-500/20 text-red-300 hover:bg-red-500/30 transition"
                >
                  <CheckCircle className="w-3 h-3" />
                  Fait — j'ai autorisé
                </button>
              </div>
            ) : (
              <button
                onClick={handleLastfmConnect}
                disabled={lfmState === "loading"}
                className="w-full flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs text-qs-text-dim hover:text-red-300 hover:bg-red-500/10 transition"
              >
                {lfmState === "loading" ? (
                  <Loader2 className="w-3 h-3 animate-spin" />
                ) : (
                  <Radio className="w-3 h-3" />
                )}
                Connecter Last.fm
              </button>
            )}

            <button
              onClick={handleLogout}
              className="w-full flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs text-qs-text-dim hover:text-red-400 hover:bg-red-500/10 transition"
            >
              <LogOut className="w-3 h-3" />
              Déconnexion
            </button>
          </div>
        )}
      </div>
    </aside>
  );
}
