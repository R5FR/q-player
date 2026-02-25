import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Disc3, Mail, Lock, Loader2 } from "lucide-react";
import { useStore } from "../store";
import * as api from "../api";

export default function LoginModal() {
  const { setSession } = useStore();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!email || !password) return;

    setLoading(true);
    setError(null);

    try {
      const session = await api.login(email, password);
      setSession(session);
    } catch (err: any) {
      setError(typeof err === "string" ? err : err.message || "Login failed");
    } finally {
      setLoading(false);
    }
  };

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-md"
      >
        <motion.div
          initial={{ scale: 0.9, y: 20 }}
          animate={{ scale: 1, y: 0 }}
          transition={{ type: "spring", damping: 25 }}
          className="glass rounded-2xl p-8 w-full max-w-md mx-4"
        >
          {/* Logo */}
          <div className="flex flex-col items-center mb-8">
            <motion.div
              animate={{ rotate: 360 }}
              transition={{ duration: 8, repeat: Infinity, ease: "linear" }}
              className="w-16 h-16 rounded-full bg-gradient-to-br from-qs-accent to-purple-600 flex items-center justify-center mb-4"
            >
              <Disc3 className="w-8 h-8 text-white" />
            </motion.div>
            <h1 className="text-3xl font-bold text-gradient">Q-Stream</h1>
            <p className="text-qs-text-dim text-sm mt-1">
              Hi-Res Music Streaming
            </p>
          </div>

          {/* Form */}
          <form onSubmit={handleLogin} className="space-y-4">
            <div className="relative">
              <Mail className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-qs-text-dim" />
              <input
                type="email"
                placeholder="Qobuz Email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="w-full pl-10 pr-4 py-3 bg-white/5 border border-white/10 rounded-xl text-white placeholder:text-qs-text-dim focus:outline-none focus:border-qs-accent/50 focus:ring-1 focus:ring-qs-accent/30 transition"
              />
            </div>
            <div className="relative">
              <Lock className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-qs-text-dim" />
              <input
                type="password"
                placeholder="Password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full pl-10 pr-4 py-3 bg-white/5 border border-white/10 rounded-xl text-white placeholder:text-qs-text-dim focus:outline-none focus:border-qs-accent/50 focus:ring-1 focus:ring-qs-accent/30 transition"
              />
            </div>

            {error && (
              <motion.p
                initial={{ opacity: 0, y: -5 }}
                animate={{ opacity: 1, y: 0 }}
                className="text-red-400 text-sm text-center"
              >
                {error}
              </motion.p>
            )}

            <button
              type="submit"
              disabled={loading || !email || !password}
              className="w-full py-3 bg-gradient-to-r from-qs-accent to-purple-600 rounded-xl font-semibold text-white hover:opacity-90 transition disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              {loading ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Connecting...
                </>
              ) : (
                "Sign in with Qobuz"
              )}
            </button>
          </form>

          <p className="text-xs text-qs-text-dim text-center mt-6">
            Requires an active Qobuz subscription for Hi-Res streaming
          </p>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  );
}
