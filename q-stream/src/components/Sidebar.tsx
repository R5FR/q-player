import { useState } from "react";
import { motion } from "framer-motion";
import { shallow } from "zustand/shallow";
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
  SlidersHorizontal,
  Settings,
  Sun,
  Moon,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-shell";
import { useStore } from "../store";
import * as api from "../api";
import type { ViewType } from "../types";

const TOP_NAV: Array<{ icon: typeof Home; label: string; view: ViewType }> = [
  { icon: Home, label: "Accueil", view: "home" },
  { icon: Search, label: "Rechercher", view: "search" },
  { icon: SlidersHorizontal, label: "Égaliseur", view: "eq" },
  { icon: Settings, label: "Paramètres", view: "settings" },
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
  { icon: FolderOpen, label: "Locale", view: "local" },
];

// ── NavBtn défini au niveau module, pas à l'intérieur de Sidebar ──
// Si NavBtn était défini dans Sidebar, chaque re-render créerait une
// nouvelle référence de fonction → React remonte les boutons → :hover perdu.
interface NavBtnProps {
  icon: typeof Home;
  label: string;
  view: ViewType;
  suffix?: React.ReactNode;
  currentView: ViewType;
  setView: (v: ViewType) => void;
}

function NavBtn({ icon: Icon, label, view, suffix, currentView, setView }: NavBtnProps) {
  return (
    <button
      onClick={() => setView(view)}
      className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-left ${
        currentView === view ? "nav-item-active" : "nav-item-inactive"
      }`}
    >
      <Icon className="w-3.5 h-3.5 flex-shrink-0 pointer-events-none" />
      <span className="font-condensed font-medium text-xs uppercase tracking-[0.1em] flex-1 truncate pointer-events-none">
        {label}
      </span>
      {suffix && <span className="ml-auto opacity-80 pointer-events-none">{suffix}</span>}
    </button>
  );
}

export default function Sidebar() {
  // Sélecteur Zustand strict : Sidebar ne re-render QUE quand ces valeurs changent.
  // Sans sélecteur, useStore() abonne au store entier → re-render toutes les 500ms.
  const { currentView, setView, session, setSession, lastfmUser, setLastfmUser, isDarkMode, toggleTheme } =
    useStore(
      (s) => ({
        currentView:   s.currentView,
        setView:       s.setView,
        session:       s.session,
        setSession:    s.setSession,
        lastfmUser:    s.lastfmUser,
        setLastfmUser: s.setLastfmUser,
        isDarkMode:    s.isDarkMode,
        toggleTheme:   s.toggleTheme,
      }),
      shallow,
    );

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

  return (
    <aside className="w-60 h-full flex flex-col flex-shrink-0 border-r border-qs-text/[0.06]">
      {/* ── Brand ── */}
      <div className="px-4 pt-5 pb-4 flex items-center justify-between flex-shrink-0">
        <div className="flex items-center gap-2.5">
          <motion.div
            whileHover={{ rotate: 360 }}
            transition={{ duration: 0.8, ease: "easeInOut" }}
            className="w-8 h-8 rounded-lg border border-qs-accent/35 bg-qs-accent/8 flex items-center justify-center flex-shrink-0"
          >
            <Disc3 className="w-4 h-4 text-qs-accent" />
          </motion.div>
          <div className="min-w-0">
            <h1 className="font-display text-xl leading-none tracking-wider text-qs-text">
              Q-STREAM
            </h1>
            <p className="font-condensed text-[8px] font-medium text-qs-accent tracking-[0.22em] uppercase mt-0.5">
              Hi-Res Audio
            </p>
          </div>
        </div>
        <motion.button
          whileTap={{ scale: 0.85 }}
          onClick={toggleTheme}
          title={isDarkMode ? "Thème clair" : "Thème sombre"}
          className="w-7 h-7 rounded-md flex items-center justify-center text-qs-text-dim hover:text-qs-text hover:bg-qs-text/5 transition-colors duration-150 flex-shrink-0"
        >
          {isDarkMode ? <Sun className="w-3.5 h-3.5" /> : <Moon className="w-3.5 h-3.5" />}
        </motion.button>
      </div>

      {/* ── Divider ── */}
      <div className="mx-4 mb-2 h-px bg-gradient-to-r from-transparent via-qs-text/8 to-transparent flex-shrink-0" />

      {/* ── Top navigation ── */}
      <nav className="px-2 pb-3 space-y-0.5 flex-shrink-0">
        {TOP_NAV.map((item) => (
          <NavBtn key={item.view} {...item} currentView={currentView} setView={setView} />
        ))}
      </nav>

      {/* ── Divider ── */}
      <div className="mx-4 mb-3 h-px bg-gradient-to-r from-transparent via-qs-text/8 to-transparent flex-shrink-0" />

      {/* ── Library section ── */}
      <div className="flex-1 flex flex-col overflow-hidden min-h-0">
        <div className="flex items-center gap-2 px-4 pb-2 flex-shrink-0">
          <Library className="w-3 h-3 text-qs-text-dim" />
          <span className="font-condensed text-[9px] font-semibold text-qs-text-dim uppercase tracking-[0.2em]">
            Bibliothèque
          </span>
        </div>

        <div className="flex-1 overflow-y-auto px-2 space-y-0.5 min-h-0 pb-2">
          {LIBRARY_NAV.map((item) => (
            <NavBtn key={item.view} {...item} currentView={currentView} setView={setView} />
          ))}
        </div>

        {/* ── User section ── */}
        {session.logged_in && (
          <div className="border-t border-qs-text/[0.06] p-3 space-y-1 flex-shrink-0">
            {/* Qobuz account */}
            <div className="flex items-center gap-2.5 px-1 py-1 mb-0.5">
              <div className="w-6 h-6 rounded-full bg-qs-accent/12 border border-qs-accent/20 flex items-center justify-center flex-shrink-0">
                <User className="w-3 h-3 text-qs-accent" />
              </div>
              <div className="flex-1 min-w-0">
                <p className="font-sans text-xs font-medium text-qs-text truncate leading-none">
                  {session.user_name}
                </p>
                {session.subscription && (
                  <p className="font-condensed text-[8px] font-medium text-qs-accent/70 tracking-wider uppercase mt-0.5">
                    {session.subscription}
                  </p>
                )}
              </div>
            </div>

            {/* Last.fm */}
            {lastfmUser ? (
              <div className="flex items-center gap-2 px-1 py-1">
                <Radio className="w-3 h-3 text-red-400 flex-shrink-0" />
                <span className="font-sans text-[11px] text-qs-text-dim truncate flex-1">
                  {lastfmUser.user_name}
                </span>
                <button
                  onClick={handleLastfmDisconnect}
                  className="font-mono text-[9px] text-qs-text-dim hover:text-qs-red transition leading-none w-4 h-4 flex items-center justify-center"
                  title="Déconnecter Last.fm"
                >
                  ✕
                </button>
              </div>
            ) : lfmState === "pending" ? (
              <div className="space-y-1.5 px-1">
                <p className="font-sans text-[10px] text-qs-text-dim leading-tight">
                  Autorise Q-Stream dans ton navigateur, puis clique sur Fait.
                </p>
                <button
                  onClick={handleLastfmDone}
                  className="w-full flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-lg font-condensed text-xs uppercase tracking-wider bg-red-500/15 text-red-300 border border-red-500/20 hover:bg-red-500/25 transition"
                >
                  <CheckCircle className="w-3 h-3" />
                  Fait — j'ai autorisé
                </button>
              </div>
            ) : (
              <button
                onClick={handleLastfmConnect}
                disabled={lfmState === "loading"}
                className="w-full flex items-center gap-2 px-3 py-1.5 rounded-lg font-condensed text-xs uppercase tracking-wider text-qs-text-dim hover:text-red-300 hover:bg-red-500/8 border border-transparent hover:border-red-500/15 transition"
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
              className="w-full flex items-center gap-2 px-3 py-1.5 rounded-lg font-condensed text-xs uppercase tracking-wider text-qs-text-dim hover:text-qs-red hover:bg-qs-red/8 border border-transparent hover:border-qs-red/15 transition"
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
