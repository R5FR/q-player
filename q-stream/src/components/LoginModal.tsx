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
          className="border-gradient-cyber rounded-2xl p-8 w-full max-w-md mx-4"
        >
          {/* Logo */}
          <div className="flex flex-col items-center mb-8">
            <motion.div
              animate={{ rotate: 360 }}
              transition={{ duration: 8, repeat: Infinity, ease: "linear" }}
              className="w-16 h-16 rounded-full border-2 border-qs-accent/40 bg-qs-accent/8 flex items-center justify-center mb-5"
              style={{ boxShadow: "0 0 28px rgb(var(--qs-accent) / 0.2)" }}
            >
              <Disc3 className="w-8 h-8 text-qs-accent" />
            </motion.div>
            <h1 className="font-display text-4xl tracking-widest text-qs-text">Q-STREAM</h1>
            <p className="font-condensed text-[10px] font-medium text-qs-accent tracking-[0.25em] uppercase mt-1">
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
                className="input-cyber w-full pl-10 pr-4 py-3"
              />
            </div>
            <div className="relative">
              <Lock className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-qs-text-dim" />
              <input
                type="password"
                placeholder="Password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="input-cyber w-full pl-10 pr-4 py-3"
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
              className="w-full py-3 bg-qs-accent rounded-xl font-condensed font-semibold text-sm uppercase tracking-widest text-black hover:bg-qs-accent-light transition disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
              style={{ boxShadow: "0 0 20px rgb(var(--qs-accent) / 0.3)" }}
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
