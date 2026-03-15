import { useEffect } from "react";
import { motion } from "framer-motion";
import { RotateCcw, Power } from "lucide-react";
import { useStore } from "../store";
import * as api from "../api";

const EQ_PRESETS: Record<string, number[]> = {
  Flat:        [0,    0,    0,    0,    0],
  Rock:        [4,    2,   -1,    2,    4],
  Jazz:        [3,    2,    0,    2,    3],
  Classical:   [4,    2,    0,   -1,    2],
  Pop:         [-1,   2,    4,    2,    0],
  Electronic:  [5,    3,    0,    2,    4],
  Vocal:       [-2,   3,    5,    3,   -2],
  "Bass Boost":[6,    4,    0,   -1,   -1],
};

export default function EqPanel() {
  const {
    eqEnabled, setEqEnabled,
    eqBands, updateEqBand, resetEq,
  } = useStore();

  // Sync EQ to backend with a short debounce so that rapid slider moves don't
  // fire dozens of sink clears (which would cause audio glitching).
  useEffect(() => {
    const timer = setTimeout(() => {
      api.setEq(eqBands, eqEnabled).catch(() => {});
    }, 150);
    return () => clearTimeout(timer);
  }, [eqBands, eqEnabled]);

  const applyPreset = (name: string) => {
    const gains = EQ_PRESETS[name];
    if (!gains) return;
    gains.forEach((gain, i) => updateEqBand(i, gain));
  };

  const EqGradient = (gain: number) => {
    if (gain === 0) return "rgba(0,212,255,0.15)";
    if (gain > 0) return `linear-gradient(to top, #00d4ff ${Math.abs(gain) / 12 * 100}%, rgba(0,212,255,0.15) ${Math.abs(gain) / 12 * 100}%)`;
    return `linear-gradient(to bottom, #8b5cf6 ${Math.abs(gain) / 12 * 100}%, rgba(139,92,246,0.15) ${Math.abs(gain) / 12 * 100}%)`;
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: 16 }}
      transition={{ type: "spring", damping: 25, stiffness: 300 }}
      className="absolute bottom-full left-1/2 -translate-x-1/2 mb-3 eq-panel z-50 p-5 w-[480px]"
    >
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-semibold text-white">Equalizer</h3>
          <button
            onClick={() => setEqEnabled(!eqEnabled)}
            className={`flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-semibold transition ${
              eqEnabled
                ? "bg-qs-accent/20 text-qs-accent border border-qs-accent/40"
                : "bg-white/5 text-qs-text-dim border border-white/10"
            }`}
          >
            <Power className="w-3 h-3" />
            {eqEnabled ? "ON" : "OFF"}
          </button>
        </div>
        <button
          onClick={resetEq}
          className="text-qs-text-dim hover:text-white transition flex items-center gap-1 text-xs"
        >
          <RotateCcw className="w-3 h-3" />
          Reset
        </button>
      </div>

      {/* EQ Sliders */}
      <div className={`flex gap-4 justify-around transition-opacity ${eqEnabled ? "opacity-100" : "opacity-40 pointer-events-none"}`}>
        {eqBands.map((band, i) => (
          <div key={band.freq} className="flex flex-col items-center gap-2">
            {/* Gain label */}
            <span className={`text-[10px] font-mono font-semibold w-8 text-center ${
              band.gain > 0 ? "text-qs-accent" : band.gain < 0 ? "text-purple-400" : "text-qs-text-dim"
            }`}>
              {band.gain > 0 ? "+" : ""}{band.gain.toFixed(1)}
            </span>

            {/* Vertical slider */}
            <div className="relative flex items-center justify-center h-[100px]">
              <input
                type="range"
                min={-12}
                max={12}
                step={0.5}
                value={band.gain}
                onChange={(e) => updateEqBand(i, parseFloat(e.target.value))}
                className="eq-slider"
                style={{
                  background: EqGradient(band.gain),
                }}
              />
              {/* Center line at 0 dB */}
              <div className="absolute top-1/2 left-1/2 -translate-x-1/2 w-3 h-px bg-white/20 pointer-events-none" />
            </div>

            {/* Frequency label */}
            <span className="text-[10px] text-qs-text-dim font-mono whitespace-nowrap">
              {band.freq >= 1000 ? `${band.freq / 1000}k` : band.freq}
              <span className="text-[8px] ml-0.5">Hz</span>
            </span>
            <span className="text-[9px] text-qs-text-dim/60">{band.label}</span>
          </div>
        ))}
      </div>

      {/* Presets */}
      <div className="mt-4 pt-3 border-t border-white/5">
        <p className="text-[10px] text-qs-text-dim uppercase tracking-wider mb-2">Presets</p>
        <div className="flex flex-wrap gap-1.5">
          {Object.keys(EQ_PRESETS).map((preset) => (
            <button
              key={preset}
              onClick={() => applyPreset(preset)}
              disabled={!eqEnabled}
              className="px-2.5 py-1 rounded-full text-[10px] font-medium transition
                bg-white/5 text-qs-text-dim border border-white/8
                hover:bg-qs-accent/10 hover:text-qs-accent hover:border-qs-accent/30
                disabled:opacity-30 disabled:cursor-not-allowed"
            >
              {preset}
            </button>
          ))}
        </div>
      </div>
    </motion.div>
  );
}
