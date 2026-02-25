import { useState } from "react";
import { motion } from "framer-motion";
import {
  Home,
  Search,
  Heart,
  ListMusic,
  FolderOpen,
  Disc3,
  LogOut,
  User,
  Sparkles,
  Radio,
  CheckCircle,
  Loader2,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-shell";
import { useStore } from "../store";
import * as api from "../api";
import type { ViewType } from "../types";

const NAV_ITEMS: Array<{ icon: typeof Home; label: string; view: ViewType }> = [
  { icon: Home, label: "Home", view: "home" },
  { icon: Search, label: "Search", view: "search" },
  { icon: Heart, label: "Favorites", view: "favorites" },
  { icon: ListMusic, label: "Queue", view: "queue" },
  { icon: FolderOpen, label: "Local Music", view: "local" },
];

export default function Sidebar() {
  const { currentView, setView, session, setSession, lastfmUser, setLastfmUser } = useStore();
  const [lfmState, setLfmState] = useState<'idle' | 'pending' | 'loading'>('idle');

  const handleLogout = async () => {
    await api.logout();
    setSession({ logged_in: false });
  };

  const handleLastfmConnect = async () => {
    try {
      setLfmState('loading');
      const url = await api.lastfmStartAuth();
      await open(url);
      setLfmState('pending');
    } catch (e) {
      console.error(e);
      setLfmState('idle');
    }
  };

  const handleLastfmDone = async () => {
    try {
      setLfmState('loading');
      const session = await api.lastfmCompleteAuth();
      setLastfmUser(session);
      setLfmState('idle');
    } catch (e) {
      console.error('Last.fm auth failed:', e);
      setLfmState('idle');
    }
  };

  const handleLastfmDisconnect = async () => {
    await api.lastfmDisconnect();
    setLastfmUser(null);
  };

  return (
    <aside className="w-64 h-full flex flex-col glass-light">
      {/* Brand */}
      <div className="p-6 flex items-center gap-3">
        <motion.div
          whileHover={{ rotate: 180 }}
          transition={{ duration: 0.6 }}
          className="w-10 h-10 rounded-xl bg-gradient-to-br from-qs-accent to-purple-600 flex items-center justify-center"
        >
          <Disc3 className="w-5 h-5 text-white" />
        </motion.div>
        <div>
          <h1 className="text-lg font-bold text-white">Q-Stream</h1>
          <p className="text-[10px] text-qs-accent-light font-medium tracking-wider uppercase">
            Hi-Res Audio
          </p>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-3 space-y-1">
        <p className="px-3 py-2 text-[10px] font-semibold text-qs-text-dim uppercase tracking-wider">
          Menu
        </p>
        {NAV_ITEMS.map(({ icon: Icon, label, view }) => (
          <motion.button
            key={view}
            whileHover={{ x: 4 }}
            whileTap={{ scale: 0.98 }}
            onClick={() => setView(view)}
            className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium transition-all ${
              currentView === view
                ? "bg-white/10 text-white"
                : "text-qs-text-dim hover:text-white hover:bg-white/5"
            }`}
          >
            <Icon className="w-4 h-4" />
            {label}
            {view === "queue" && (
              <Sparkles className="w-3 h-3 ml-auto text-amber-400" />
            )}
          </motion.button>
        ))}
      </nav>

      {/* User */}
      {session.logged_in && (
        <div className="p-4 border-t border-white/5 space-y-3">
          {/* Qobuz user */}
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-full bg-qs-accent/20 flex items-center justify-center">
              <User className="w-4 h-4 text-qs-accent-light" />
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-white truncate">
                {session.user_name}
              </p>
              {session.subscription && (
                <p className="text-[10px] text-qs-accent-light">
                  {session.subscription}
                </p>
              )}
            </div>
          </div>

          {/* Last.fm */}
          {lastfmUser ? (
            <div className="flex items-center gap-2 px-1">
              <Radio className="w-3.5 h-3.5 text-red-400 flex-shrink-0" />
              <span className="text-xs text-white/70 truncate flex-1">{lastfmUser.user_name}</span>
              <button
                onClick={handleLastfmDisconnect}
                className="text-[10px] text-qs-text-dim hover:text-red-400 transition"
                title="Disconnect Last.fm"
              >
                ✕
              </button>
            </div>
          ) : lfmState === 'pending' ? (
            <div className="space-y-1.5 px-1">
              <p className="text-[10px] text-qs-text-dim leading-tight">
                Authorize Q-Stream in your browser, then click Done.
              </p>
              <button
                onClick={handleLastfmDone}
                className="w-full flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg text-xs bg-red-500/20 text-red-300 hover:bg-red-500/30 transition"
              >
                <CheckCircle className="w-3.5 h-3.5" />
                Done — I authorized
              </button>
            </div>
          ) : (
            <button
              onClick={handleLastfmConnect}
              disabled={lfmState === 'loading'}
              className="w-full flex items-center gap-2 px-3 py-2 rounded-lg text-xs text-qs-text-dim hover:text-red-300 hover:bg-red-500/10 transition"
            >
              {lfmState === 'loading' ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <Radio className="w-3.5 h-3.5" />
              )}
              Connect Last.fm
            </button>
          )}

          <button
            onClick={handleLogout}
            className="w-full flex items-center gap-2 px-3 py-2 rounded-lg text-xs text-qs-text-dim hover:text-red-400 hover:bg-red-500/10 transition"
          >
            <LogOut className="w-3.5 h-3.5" />
            Sign out
          </button>
        </div>
      )}
    </aside>
  );
}
