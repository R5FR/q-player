import { useEffect, useRef, useCallback, useState } from "react";
import { RotateCcw, Power, Zap } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";
import type { EqBand } from "../../types";
// ── Presets ───────────────────────────────────────────────────────────────────

const PRESETS_5: Record<string, number[]> = {
  Flat:        [0,    0,    0,    0,    0],
  Rock:        [4,    2,   -1,    2,    4],
  Jazz:        [3,    2,    0,    2,    3],
  Classical:   [4,    2,    0,   -1,    2],
  Pop:         [-1,   2,    4,    2,    0],
  Electronic:  [5,    3,    0,    2,    4],
  Vocal:       [-2,   3,    5,    3,   -2],
  "Bass Boost":[6,    4,    0,   -1,   -1],
};

const PRESETS_10: Record<string, number[]> = {
  Flat:        [0,  0,  0,  0,  0,  0,  0,  0,  0,  0],
  Rock:        [5,  4,  3,  1, -1,  0,  1,  3,  4,  4],
  Jazz:        [3,  3,  2,  2,  0,  0,  1,  2,  3,  3],
  Classical:   [5,  4,  3,  2,  0, -1, -1,  0,  2,  3],
  Pop:         [-1,-1,  0,  2,  4,  5,  4,  2,  1,  0],
  Electronic:  [6,  5,  3,  0,  0,  1,  2,  3,  4,  4],
  Vocal:       [-2,-2, -1,  2,  5,  6,  5,  3,  1, -1],
  "Bass Boost":[8,  7,  5,  3,  1,  0, -1, -1, -1, -1],
  "Hi-Fi":     [2,  1,  0, -1, -1, -1,  0,  1,  2,  3],
  Lounge:      [3,  2,  1,  0, -1,  0,  1,  2,  3,  2],
};

// ── Exact biquad peaking EQ frequency response ────────────────────────────────

function peakDb(f: number, fc: number, gainDb: number, q: number, sr = 44100): number {
  if (Math.abs(gainDb) < 0.001) return 0;
  const A = Math.pow(10, gainDb / 40);
  const w0 = (2 * Math.PI * fc) / sr;
  const alpha = Math.sin(w0) / (2 * q);
  const b0 = 1 + alpha * A, b1 = -2 * Math.cos(w0), b2 = 1 - alpha * A;
  const a0 = 1 + alpha / A, a1 = b1, a2 = 1 - alpha / A;
  const w = (2 * Math.PI * f) / sr;
  const c1 = Math.cos(w), s1 = Math.sin(w), c2 = Math.cos(2 * w), s2 = Math.sin(2 * w);
  const nr = b0 + b1 * c1 + b2 * c2, ni = -(b1 * s1 + b2 * s2);
  const dr = a0 + a1 * c1 + a2 * c2, di = -(a1 * s1 + a2 * s2);
  const mag = Math.sqrt((nr * nr + ni * ni) / (dr * dr + di * di));
  return 20 * Math.log10(Math.max(mag, 1e-10));
}

function combinedDb(f: number, bands: EqBand[]): number {
  return bands.reduce((s, b) => s + peakDb(f, b.freq, b.gain, b.q), 0);
}

// ── Real-time spectrum canvas (data from Rust FFT via Tauri event) ────────────

// dB-scale a linear FFT magnitude to [0, 1]
// Rust normalises to ~[0, 0.5] for full scale → 0 dB ≡ 0.5
const DB_FLOOR = -70; // dB noise floor
const DB_REF   =  0;  // dB top (0 dB ≡ linear 1.0 ≡ full scale)
function linToDB(v: number): number {
  return 20 * Math.log10(Math.max(v, 1e-9));
}
function dbToNorm(dB: number): number {
  return Math.max(0, Math.min(1, (dB - DB_FLOOR) / (DB_REF - DB_FLOOR)));
}

function SpectrumCanvas() {
  const canvasRef   = useRef<HTMLCanvasElement>(null);
  const smoothRef   = useRef(new Float32Array(80).fill(0));
  const rawRef      = useRef(new Float32Array(80).fill(0));
  const rafRef      = useRef(0);
  const lastDataRef = useRef(new Float32Array(80).fill(0));
  const staleCount  = useRef(0);

  useEffect(() => {
    let alive = true;
    const timer = setInterval(() => {
      if (!alive) return;
      api.getSpectrum()
        .then(data => {
          if (!alive || !data?.length) return;
          let different = false;
          for (let i = 0; i < Math.min(data.length, 80); i++) {
            if (data[i] !== lastDataRef.current[i]) { different = true; break; }
          }
          if (different) {
            staleCount.current = 0;
            for (let i = 0; i < Math.min(data.length, rawRef.current.length); i++) {
              rawRef.current[i]      = data[i];
              lastDataRef.current[i] = data[i];
            }
          } else {
            staleCount.current++;
            if (staleCount.current > 10) rawRef.current.fill(0);
          }
        })
        .catch(() => {});
    }, 50);
    return () => { alive = false; clearInterval(timer); };
  }, []);

  // ── Resize canvas buffer to match CSS display size ──────────────────
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ro = new ResizeObserver(() => {
      canvas.width  = canvas.clientWidth  * window.devicePixelRatio;
      canvas.height = canvas.clientHeight * window.devicePixelRatio;
    });
    ro.observe(canvas);
    return () => ro.disconnect();
  }, []);

  // ── Animation loop ──────────────────────────────────────────────────
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d")!;
    const N = 80;

    const draw = () => {
      rafRef.current = requestAnimationFrame(draw);
      const W = canvas.width  || canvas.clientWidth;
      const H = canvas.height || canvas.clientHeight;
      if (W === 0 || H === 0) return;

      ctx.clearRect(0, 0, W, H);

      const barW = W / N;
      for (let i = 0; i < N; i++) {
        // Convert linear → dB → [0,1] with gamma lift
        const linear  = rawRef.current[i];
        const target  = Math.pow(dbToNorm(linToDB(linear)), 0.75); // gamma lift

        // Fast attack, slow decay
        if (target > smoothRef.current[i]) {
          smoothRef.current[i] += (target - smoothRef.current[i]) * 0.55;
        } else {
          smoothRef.current[i] += (target - smoothRef.current[i]) * 0.07;
        }

        const h = Math.max(2, smoothRef.current[i] * (H - 4));
        const x = i * barW;

        // Colour: lime (bass) → orange (treble) — matches --qs-accent / --qs-accent-2
        const t = i / (N - 1);
        const r = Math.round(183 + (255 - 183) * t);
        const g = Math.round(255 + (100 - 255) * t);
        const b = Math.round(46  + (50  - 46)  * t);

        const grad = ctx.createLinearGradient(0, H - h, 0, H);
        grad.addColorStop(0, `rgba(${r},${g},${b},0.85)`);
        grad.addColorStop(1, `rgba(${r},${g},${b},0.08)`);
        ctx.fillStyle = grad;

        const bx = x + 1, by = H - h, bw = Math.max(1, barW - 2);
        const rx = Math.min(2, bw / 2);
        ctx.beginPath();
        ctx.moveTo(bx + rx, by);
        ctx.lineTo(bx + bw - rx, by);
        ctx.quadraticCurveTo(bx + bw, by, bx + bw, by + rx);
        ctx.lineTo(bx + bw, by + h);
        ctx.lineTo(bx, by + h);
        ctx.lineTo(bx, by + rx);
        ctx.quadraticCurveTo(bx, by, bx + rx, by);
        ctx.closePath();
        ctx.fill();
      }
    };

    draw();
    return () => cancelAnimationFrame(rafRef.current);
  }, []);

  return (
    <canvas
      ref={canvasRef}
      style={{ display: "block", width: "100%", height: "100%" }}
    />
  );
}

// ── EQ Curve Display ──────────────────────────────────────────────────────────

const FREQ_LABELS = [
  { f: 20, label: "20" }, { f: 50, label: "50" }, { f: 100, label: "100" },
  { f: 200, label: "200" }, { f: 500, label: "500" }, { f: 1000, label: "1k" },
  { f: 2000, label: "2k" }, { f: 5000, label: "5k" }, { f: 10000, label: "10k" },
  { f: 20000, label: "20k" },
];
const DB_LINES = [12, 6, 0, -6, -12];
const MIN_F = 20, MAX_F = 20000, MAX_DB = 12;
const W = 1000, H = 280, PAD_L = 36, PAD_R = 16, PAD_T = 16, PAD_B = 28;
const INNER_W = W - PAD_L - PAD_R, INNER_H = H - PAD_T - PAD_B;
const N_PTS = 400;

function freqToX(f: number): number {
  return PAD_L + (Math.log10(f / MIN_F) / Math.log10(MAX_F / MIN_F)) * INNER_W;
}
function dbToY(db: number): number {
  return PAD_T + (1 - (db + MAX_DB) / (2 * MAX_DB)) * INNER_H;
}

function EqDisplay({ bands, enabled }: { bands: EqBand[]; enabled: boolean }) {
  const pts = Array.from({ length: N_PTS }, (_, i) => {
    const t = i / (N_PTS - 1);
    const f = MIN_F * Math.pow(MAX_F / MIN_F, t);
    const db = enabled ? combinedDb(f, bands) : 0;
    return [freqToX(f), dbToY(Math.max(-MAX_DB, Math.min(MAX_DB, db)))] as [number, number];
  });

  const linePath = `M ${pts.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" L ")}`;
  const y0 = dbToY(0);
  const fillPath = `M ${PAD_L},${y0} L ${pts.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" L ")} L ${PAD_L + INNER_W},${y0} Z`;

  const activeDots = enabled ? bands.filter((b) => Math.abs(b.gain) > 0.1) : [];

  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="w-full h-full" preserveAspectRatio="xMidYMid meet">
      <defs>
        <linearGradient id="curve-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="rgb(183,255,46)" stopOpacity={enabled ? 0.20 : 0.04} />
          <stop offset="50%" stopColor="rgb(183,255,46)" stopOpacity={enabled ? 0.05 : 0.01} />
          <stop offset="100%" stopColor="rgb(183,255,46)" stopOpacity={0} />
        </linearGradient>
        <linearGradient id="curve-stroke" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor="rgb(255,100,50)" />
          <stop offset="50%" stopColor="rgb(183,255,46)" />
          <stop offset="100%" stopColor="rgb(183,255,46)" />
        </linearGradient>
        <filter id="glow-line">
          <feGaussianBlur stdDeviation="3" result="blur" />
          <feMerge><feMergeNode in="blur" /><feMergeNode in="SourceGraphic" /></feMerge>
        </filter>
        <filter id="glow-dot">
          <feGaussianBlur stdDeviation="3" result="blur" />
          <feMerge><feMergeNode in="blur" /><feMergeNode in="SourceGraphic" /></feMerge>
        </filter>
        <clipPath id="chart-clip">
          <rect x={PAD_L} y={PAD_T} width={INNER_W} height={INNER_H} />
        </clipPath>
      </defs>

      {/* ── Background grid ── */}
      {DB_LINES.map((db) => {
        const y = dbToY(db);
        return (
          <g key={db}>
            <line x1={PAD_L} y1={y} x2={PAD_L + INNER_W} y2={y}
              stroke={db === 0 ? "rgba(255,255,255,0.15)" : "rgba(255,255,255,0.05)"}
              strokeWidth={db === 0 ? 1.5 : 1}
              strokeDasharray={db === 0 ? undefined : "3 5"} />
            <text x={PAD_L - 6} y={y + 3.5} textAnchor="end"
              fontSize={9} fill="rgba(255,255,255,0.25)" fontFamily="monospace">
              {db > 0 ? `+${db}` : db}
            </text>
          </g>
        );
      })}

      {/* Freq grid + labels */}
      {FREQ_LABELS.map(({ f, label }) => {
        const x = freqToX(f);
        return (
          <g key={f}>
            <line x1={x} y1={PAD_T} x2={x} y2={PAD_T + INNER_H}
              stroke="rgba(255,255,255,0.04)" strokeWidth={1} />
            <text x={x} y={H - 6} textAnchor="middle"
              fontSize={9} fill="rgba(255,255,255,0.25)" fontFamily="monospace">
              {label}
            </text>
          </g>
        );
      })}

      {/* Axis frame */}
      <rect x={PAD_L} y={PAD_T} width={INNER_W} height={INNER_H}
        fill="none" stroke="rgba(255,255,255,0.06)" strokeWidth={1} />

      {/* ── Curve ── */}
      <g clipPath="url(#chart-clip)">
        <path d={fillPath} fill="url(#curve-fill)" />
        <path d={linePath} fill="none"
          stroke={enabled ? "url(#curve-stroke)" : "rgba(255,255,255,0.15)"}
          strokeWidth={enabled ? 2.5 : 1.5}
          filter={enabled ? "url(#glow-line)" : undefined} />
      </g>

      {/* Band markers */}
      {activeDots.map((b) => {
        const x = freqToX(b.freq);
        const db = enabled ? combinedDb(b.freq, bands) : 0;
        const y = dbToY(Math.max(-MAX_DB, Math.min(MAX_DB, db)));
        const col = b.gain > 0 ? "rgb(183,255,46)" : "rgb(255,100,50)";
        return (
          <g key={b.freq} clipPath="url(#chart-clip)">
            <line x1={x} y1={dbToY(0)} x2={x} y2={y}
              stroke={col} strokeWidth={1} strokeOpacity={0.3} strokeDasharray="2 3" />
            <circle cx={x} cy={y} r={5} fill={col} opacity={0.9}
              filter="url(#glow-dot)" />
            <circle cx={x} cy={y} r={3} fill="white" opacity={0.9} />
          </g>
        );
      })}
    </svg>
  );
}

// ── Vertical Slider ───────────────────────────────────────────────────────────

function BandSlider({
  band, index, onChange, compact,
}: {
  band: EqBand; index: number; onChange: (i: number, v: number) => void; compact?: boolean;
}) {
  const [dragging, setDragging] = useState(false);
  const isPos = band.gain > 0.01;
  const isNeg = band.gain < -0.01;
  const absR = Math.abs(band.gain) / 12;

  const trackBg = isPos
    ? `linear-gradient(to top, rgb(183,255,46) ${absR * 50}%, rgba(183,255,46,0.1) ${absR * 50}%)`
    : isNeg
    ? `linear-gradient(to bottom, rgb(255,100,50) ${absR * 50}%, rgba(255,100,50,0.1) ${absR * 50}%)`
    : "rgba(255,255,255,0.06)";

  const labelColor = isPos ? "rgb(183,255,46)" : isNeg ? "rgb(255,100,50)" : "rgba(255,255,255,0.3)";
  const sliderH = compact ? "h-24" : "h-32";

  return (
    <div className="flex flex-col items-center gap-2 flex-1 min-w-0 select-none">
      {/* dB label */}
      <span
        className="text-[11px] font-mono font-bold tabular-nums transition-colors duration-150"
        style={{ color: labelColor }}
      >
        {band.gain > 0 ? "+" : ""}{band.gain.toFixed(1)}
      </span>

      {/* Track + thumb */}
      <div className={`relative flex items-center justify-center ${sliderH} w-full`}>
        {/* Center line */}
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 w-3/4 h-px bg-white/10 pointer-events-none z-10" />
        <input
          type="range"
          min={-12} max={12} step={0.5}
          value={band.gain}
          onPointerDown={() => setDragging(true)}
          onPointerUp={() => setDragging(false)}
          onChange={(e) => onChange(index, parseFloat(e.target.value))}
          className={`eq-slider transition-all ${dragging ? "scale-110" : ""}`}
          style={{ background: trackBg }}
        />
      </div>

      {/* Freq + label */}
      <div className="text-center leading-none space-y-0.5">
        <p className="text-[11px] font-semibold font-mono"
          style={{ color: dragging ? "#fff" : "rgba(255,255,255,0.7)" }}>
          {band.freq >= 1000 ? `${band.freq / 1000}k` : band.freq}
        </p>
        <p className="text-[9px] text-qs-text-dim">{band.label}</p>
      </div>
    </div>
  );
}

// ── Main ──────────────────────────────────────────────────────────────────────

export default function EqView() {
  const {
    eqEnabled, setEqEnabled,
    eqBands, updateEqBand, resetEq,
    eqAdvanced, setEqAdvanced,
    eqBandsAdvanced, updateEqBandAdvanced, resetEqAdvanced,
  } = useStore();

  const activeBands = eqAdvanced ? eqBandsAdvanced : eqBands;
  const updateBand  = eqAdvanced ? updateEqBandAdvanced : updateEqBand;
  const resetBands  = eqAdvanced ? resetEqAdvanced : resetEq;
  const presets     = eqAdvanced ? PRESETS_10 : PRESETS_5;

  const syncRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const syncEq = useCallback(() => {
    if (syncRef.current) clearTimeout(syncRef.current);
    syncRef.current = setTimeout(() => {
      api.setEq(activeBands, eqEnabled).catch(() => {});
    }, 120);
  }, [activeBands, eqEnabled]);

  useEffect(() => { syncEq(); }, [syncEq]);

  const applyPreset = (name: string) => {
    presets[name]?.forEach((gain, i) => updateBand(i, gain));
  };

  const activePreset = Object.keys(presets).find((name) =>
    presets[name].every((g, i) => g === (activeBands[i]?.gain ?? 0))
  ) ?? null;

  return (
    <div className="flex flex-col h-full overflow-y-auto">
      <div className="flex flex-col gap-8 p-8 min-h-full">

        {/* ── Header ── */}
        <div className="flex items-start justify-between gap-4">
          <div>
            <h1 className="text-3xl font-bold text-white tracking-tight">Égaliseur</h1>
            <p className="text-qs-text-dim text-sm mt-1">Courbe de réponse en fréquence — calcul exact du filtre biquad</p>
          </div>
          <div className="flex items-center gap-3 flex-shrink-0 mt-1">
            {/* Mode toggle */}
            <div className="flex items-center gap-1 p-1 rounded-xl bg-black/30 border border-white/8">
              <button
                onClick={() => setEqAdvanced(false)}
                className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all duration-200 ${
                  !eqAdvanced
                    ? "bg-qs-accent/20 text-qs-accent shadow-neon-sm"
                    : "text-qs-text-dim hover:text-white/70"
                }`}
              >
                5 bandes
              </button>
              <button
                onClick={() => setEqAdvanced(true)}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all duration-200 ${
                  eqAdvanced
                    ? "bg-qs-accent-2/20 text-qs-accent-2 shadow-neon-orange"
                    : "text-qs-text-dim hover:text-white/70"
                }`}
              >
                <Zap className="w-3 h-3" />
                10 bandes
              </button>
            </div>

            {/* Reset */}
            <button
              onClick={resetBands}
              title="Réinitialiser"
              className="w-9 h-9 rounded-xl flex items-center justify-center text-qs-text-dim hover:text-white hover:bg-white/8 transition-all border border-white/8"
            >
              <RotateCcw className="w-4 h-4" />
            </button>

            {/* Power */}
            <button
              onClick={() => setEqEnabled(!eqEnabled)}
              className={`flex items-center gap-2 px-5 py-2.5 rounded-xl text-sm font-semibold transition-all duration-200 border ${
                eqEnabled
                  ? "bg-qs-accent/15 text-qs-accent border-qs-accent/35 shadow-neon-sm"
                  : "bg-white/4 text-qs-text-dim border-white/10 hover:border-white/20 hover:text-white/70"
              }`}
            >
              <Power className="w-4 h-4" />
              {eqEnabled ? "Activé" : "Désactivé"}
            </button>
          </div>
        </div>

        {/* ── Spectrum ── */}
        <div className="relative rounded-2xl overflow-hidden border border-white/8 bg-black/30" style={{ height: 120 }}>
          <div className="absolute inset-0 pointer-events-none opacity-30"
            style={{ background: "repeating-linear-gradient(0deg, transparent, transparent 3px, rgba(0,0,0,0.04) 3px, rgba(0,0,0,0.04) 4px)" }} />
          <SpectrumCanvas />
          {/* Freq labels */}
          <div className="absolute bottom-1.5 left-0 right-0 flex justify-between px-3 pointer-events-none">
            {["20", "50", "100", "200", "500", "1k", "2k", "5k", "10k", "20k"].map((f) => (
              <span key={f} className="text-[8px] text-white/20 font-mono">{f}</span>
            ))}
          </div>
        </div>

        {/* ── EQ Curve ── */}
        <div className="relative rounded-2xl overflow-hidden border border-white/8 bg-black/25"
          style={{ minHeight: 220 }}>
          {/* Subtle scanline overlay */}
          <div className="absolute inset-0 pointer-events-none"
            style={{ background: "repeating-linear-gradient(0deg, transparent, transparent 3px, rgba(0,0,0,0.03) 3px, rgba(0,0,0,0.03) 4px)" }} />
          <div className="p-4 h-full" style={{ minHeight: 220 }}>
            <EqDisplay bands={activeBands} enabled={eqEnabled} />
          </div>
        </div>

        {/* ── Sliders ── */}
        <div className={`transition-all duration-300 ${eqEnabled ? "opacity-100" : "opacity-35 pointer-events-none"}`}>
          <div className="glass rounded-2xl border border-white/8 p-6">
            <div className={`flex items-stretch justify-around ${eqAdvanced ? "gap-1" : "gap-3"}`}>
              {activeBands.map((band, i) => (
                <BandSlider
                  key={`${band.freq}-${eqAdvanced}`}
                  band={band}
                  index={i}
                  onChange={updateBand}
                  compact={eqAdvanced}
                />
              ))}
            </div>
          </div>
        </div>

        {/* ── Presets ── */}
        <div>
          <p className="text-[10px] font-semibold uppercase tracking-widest text-qs-text-dim mb-3">
            Préréglages
          </p>
          <div className="flex flex-wrap gap-2">
            {Object.keys(presets).map((name) => (
              <button
                key={name}
                onClick={() => applyPreset(name)}
                disabled={!eqEnabled}
                className={`px-4 py-2 rounded-xl text-xs font-semibold transition-all duration-150 border disabled:opacity-25 disabled:cursor-not-allowed ${
                  activePreset === name
                    ? "bg-qs-accent/15 text-qs-accent border-qs-accent/35 shadow-neon-sm"
                    : "bg-white/3 text-qs-text-dim border-white/8 hover:bg-white/7 hover:text-white/80 hover:border-white/15"
                }`}
              >
                {name}
              </button>
            ))}
          </div>
        </div>

      </div>
    </div>
  );
}
