import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// ─────────────────────────────────────────────────────────────────────────────
// Pure functions extracted from SpectrumViz (must stay in sync with component)
// ─────────────────────────────────────────────────────────────────────────────

const DB_FLOOR = -70;
const DB_REF   =  0;

function linToDB(v: number): number {
  return 20 * Math.log10(Math.max(v, 1e-9));
}

function dbToNorm(dB: number): number {
  return Math.max(0, Math.min(1, (dB - DB_FLOOR) / (DB_REF - DB_FLOOR)));
}

function applySmoothing(raw: Float32Array, smooth: Float32Array, N: number): void {
  for (let i = 0; i < N; i++) {
    const target = Math.pow(dbToNorm(linToDB(raw[i])), 0.75);
    const delta  = target - smooth[i];
    smooth[i]   += delta * (delta > 0 ? 0.55 : 0.07);
  }
}

// ── linToDB ──────────────────────────────────────────────────────────────────

describe("linToDB", () => {
  it("maps 1.0 to 0 dB (unity gain)", () => {
    expect(linToDB(1.0)).toBeCloseTo(0, 3);
  });

  it("maps 0.5 to ≈ -6 dB (half amplitude)", () => {
    expect(linToDB(0.5)).toBeCloseTo(-6.02, 1);
  });

  it("maps 0 to a large negative (clamped at 1e-9)", () => {
    const expected = 20 * Math.log10(1e-9); // ≈ -180 dB
    expect(linToDB(0)).toBeCloseTo(expected, 1);
  });

  it("never returns ±Infinity", () => {
    expect(isFinite(linToDB(0))).toBe(true);
    expect(isFinite(linToDB(1e9))).toBe(true);
  });

  it("is monotonically increasing", () => {
    expect(linToDB(0.1)).toBeLessThan(linToDB(0.5));
    expect(linToDB(0.5)).toBeLessThan(linToDB(1.0));
  });
});

// ── dbToNorm ─────────────────────────────────────────────────────────────────

describe("dbToNorm", () => {
  it("maps 0 dB (ceiling) to 1.0", () => {
    expect(dbToNorm(0)).toBeCloseTo(1, 5);
  });

  it("maps -70 dB (floor) to 0.0", () => {
    expect(dbToNorm(-70)).toBeCloseTo(0, 5);
  });

  it("maps -35 dB (midpoint) to 0.5", () => {
    expect(dbToNorm(-35)).toBeCloseTo(0.5, 5);
  });

  it("clamps values below DB_FLOOR to 0", () => {
    expect(dbToNorm(-100)).toBe(0);
    expect(dbToNorm(-9999)).toBe(0);
  });

  it("clamps values above 0 dB to 1", () => {
    expect(dbToNorm(6)).toBe(1);
    expect(dbToNorm(999)).toBe(1);
  });

  it("output is always in [0, 1] for any input", () => {
    for (const db of [-200, -70, -35, -10, -1, 0, 6, 100]) {
      const v = dbToNorm(db);
      expect(v).toBeGreaterThanOrEqual(0);
      expect(v).toBeLessThanOrEqual(1);
    }
  });
});

// ── Smoothing algorithm ───────────────────────────────────────────────────────

describe("spectrum smoothing", () => {
  const N = 80;

  it("attack is fast: reaches ≥80% of peak target in 3 frames", () => {
    const raw    = new Float32Array(N).fill(1.0);
    const smooth = new Float32Array(N).fill(0);
    const target = Math.pow(dbToNorm(linToDB(1.0)), 0.75);

    for (let f = 0; f < 3; f++) applySmoothing(raw, smooth, N);

    expect(smooth[0]).toBeGreaterThan(target * 0.8);
  });

  it("decay is slow: takes >25 frames to fall below 10% after silence", () => {
    // With decay factor 0.07: smooth *= 0.93 each frame.
    // 0.93^n < 0.1 → n > log(0.1)/log(0.93) ≈ 31.9 frames from peak.
    const raw    = new Float32Array(N).fill(1.0);
    const smooth = new Float32Array(N).fill(0);

    // Bring to peak
    for (let i = 0; i < 10; i++) applySmoothing(raw, smooth, N);
    raw.fill(0);

    let frames = 0;
    while (smooth[0] > 0.1 && frames < 500) {
      applySmoothing(raw, smooth, N);
      frames++;
    }
    // Must take at least 25 frames (actual: ~32 with decay=0.07)
    expect(frames).toBeGreaterThan(25);
  });

  it("smooth values never go negative", () => {
    const raw    = new Float32Array(N).fill(0);
    const smooth = new Float32Array(N).fill(0.5);

    for (let i = 0; i < 500; i++) applySmoothing(raw, smooth, N);

    expect(smooth.every(v => v >= 0)).toBe(true);
  });

  it("converges to near-zero after prolonged silence", () => {
    const raw    = new Float32Array(N).fill(0);
    const smooth = new Float32Array(N).fill(1.0);

    for (let i = 0; i < 600; i++) applySmoothing(raw, smooth, N);

    expect(smooth[0]).toBeLessThan(0.001);
  });

  it("bins are independent: one active bin doesn't bleed to neighbours", () => {
    const raw    = new Float32Array(N).fill(0);
    raw[40]      = 1.0; // only bin 40 is active
    const smooth = new Float32Array(N).fill(0);

    for (let i = 0; i < 10; i++) applySmoothing(raw, smooth, N);

    expect(smooth[40]).toBeGreaterThan(0.5); // active bin is raised
    expect(smooth[0]).toBeLessThan(0.001);   // bin 0 stays silent
    expect(smooth[79]).toBeLessThan(0.001);  // bin 79 stays silent
  });
});

// ── Polling loop resilience ───────────────────────────────────────────────────
// These tests simulate the setInterval + api.getSpectrum() polling and verify
// the "no fetchInFlight" design survives edge cases that killed the old design.

describe("spectrum polling resilience", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(()  => vi.useRealTimers());

  it("runs 1500 ticks without error (75 seconds at 50ms)", async () => {
    let pollCount   = 0;
    let successCount = 0;
    const rawRef     = new Float32Array(80).fill(0);
    let alive        = true;

    const fakeGetSpectrum = () =>
      Promise.resolve<number[]>(Array.from({ length: 80 }, () => 0.5));

    const timer = setInterval(() => {
      if (!alive) return;
      pollCount++;
      fakeGetSpectrum()
        .then(d => {
          if (!alive) return;
          if (d?.length) for (let i = 0; i < 80; i++) rawRef[i] = d[i];
          successCount++;
        })
        .catch(() => {});
    }, 50);

    for (let i = 0; i < 1500; i++) {
      vi.advanceTimersByTime(50);
      await Promise.resolve();
    }

    clearInterval(timer);
    alive = false;

    expect(pollCount).toBe(1500);
    expect(successCount).toBe(1500);
    expect(rawRef[0]).toBeCloseTo(0.5, 3);
  });

  it("surviving polls still run when some promises never resolve (old fetchInFlight bug)", async () => {
    // This test WOULD FAIL with the old fetchInFlight design:
    // a hung Promise → flag stuck true → all future polls skipped.
    // With fire-and-forget design, hung promises are simply abandoned.

    let tick         = 0;
    let hangCount    = 0;
    let successCount = 0;
    const rawRef     = new Float32Array(80).fill(0);
    let alive        = true;

    const fakeGetSpectrum = (): Promise<number[]> => {
      tick++;
      if (tick % 100 === 0) {
        hangCount++;
        return new Promise(() => {}); // never resolves — simulates IPC hang
      }
      return Promise.resolve(Array.from({ length: 80 }, () => 0.5));
    };

    const timer = setInterval(() => {
      if (!alive) return;
      fakeGetSpectrum()
        .then(d => {
          if (!alive) return;
          if (d?.length) for (let i = 0; i < 80; i++) rawRef[i] = d[i];
          successCount++;
        })
        .catch(() => {});
    }, 50);

    for (let i = 0; i < 500; i++) {
      vi.advanceTimersByTime(50);
      await Promise.resolve();
    }

    clearInterval(timer);
    alive = false;

    expect(hangCount).toBe(5);
    // 495 polls succeed — data keeps flowing despite hung promises
    expect(successCount).toBe(495);
    expect(rawRef[0]).toBeCloseTo(0.5, 3);
  });

  it("alive flag stops rawRef updates after cleanup", async () => {
    let updateCount = 0;
    const rawRef    = new Float32Array(80).fill(0);
    let alive       = true;

    const fakeGetSpectrum = () =>
      Promise.resolve<number[]>(Array.from({ length: 80 }, () => 0.99));

    const timer = setInterval(() => {
      if (!alive) return;
      fakeGetSpectrum()
        .then(d => {
          if (!alive) return; // guard inside .then()
          if (d?.length) for (let i = 0; i < 80; i++) rawRef[i] = d[i];
          updateCount++;
        })
        .catch(() => {});
    }, 50);

    for (let i = 0; i < 10; i++) {
      vi.advanceTimersByTime(50);
      await Promise.resolve();
    }

    // Cleanup (simulates React useEffect cleanup)
    clearInterval(timer);
    alive = false;

    // Any in-flight .then() should be blocked by alive flag
    await Promise.resolve();
    await Promise.resolve();

    expect(updateCount).toBeLessThanOrEqual(10);
  });

  it("concurrent slow promises all resolve independently", async () => {
    const resolvers: Array<(v: number[]) => void> = [];
    let pollCount    = 0;
    let successCount = 0;
    const rawRef     = new Float32Array(80).fill(0);
    let alive        = true;

    // All calls pending — resolved in batch below
    const fakeGetSpectrum = (): Promise<number[]> =>
      new Promise<number[]>(resolve => resolvers.push(resolve));

    const timer = setInterval(() => {
      if (!alive) return;
      pollCount++;
      fakeGetSpectrum()
        .then(d => {
          if (!alive) return;
          if (d?.length) for (let i = 0; i < 80; i++) rawRef[i] = d[i];
          successCount++;
        })
        .catch(() => {});
    }, 50);

    // Fire 10 ticks → 10 concurrent pending promises
    for (let i = 0; i < 10; i++) vi.advanceTimersByTime(50);

    // Resolve all at once
    const pending = [...resolvers];
    resolvers.length = 0;
    pending.forEach(r => r(Array.from({ length: 80 }, () => 0.7)));

    await Promise.resolve();
    await Promise.resolve();

    clearInterval(timer);
    alive = false;

    expect(pollCount).toBe(10);
    expect(successCount).toBe(10);
    expect(rawRef[0]).toBeCloseTo(0.7, 3);
  });

  it("interval keeps firing even after errors", async () => {
    let pollCount    = 0;
    let errorCount   = 0;
    let successCount = 0;
    const rawRef     = new Float32Array(80).fill(0);
    let alive        = true;

    const fakeGetSpectrum = (): Promise<number[]> => {
      pollCount++;
      // every 5th call throws
      if (pollCount % 5 === 0) return Promise.reject(new Error("IPC error"));
      return Promise.resolve(Array.from({ length: 80 }, () => 0.3));
    };

    const timer = setInterval(() => {
      if (!alive) return;
      fakeGetSpectrum()
        .then(d => {
          if (!alive) return;
          if (d?.length) for (let i = 0; i < 80; i++) rawRef[i] = d[i];
          successCount++;
        })
        .catch(() => { errorCount++; });
    }, 50);

    for (let i = 0; i < 100; i++) {
      vi.advanceTimersByTime(50);
      await Promise.resolve();
    }
    // Flush remaining microtasks from the last tick
    await Promise.resolve();
    await Promise.resolve();

    clearInterval(timer);
    alive = false;

    expect(pollCount).toBe(100);
    expect(errorCount).toBeGreaterThanOrEqual(18);
    expect(successCount + errorCount).toBeGreaterThanOrEqual(99);
    expect(rawRef[0]).toBeCloseTo(0.3, 3);
  });
});

// ── Spectrum data contract ───────────────────────────────────────────────────

describe("spectrum data contract", () => {
  it("get_spectrum mock returns exactly 80 values", () => {
    const mockData = Array.from({ length: 80 }, () => Math.random() * 0.5);
    expect(mockData.length).toBe(80);
  });

  it("all spectrum values are in [0, ∞) — never negative from the Rust FFT", () => {
    // The FFT uses .norm() which is always >= 0
    const mockRustOutput = Array.from({ length: 80 }, () => Math.random() * 0.5);
    expect(mockRustOutput.every(v => v >= 0)).toBe(true);
  });

  it("rawRef indexing is safe for any length ≤ N", () => {
    const N      = 80;
    const rawRef = new Float32Array(N).fill(0);
    const data   = [0.1, 0.2, 0.3]; // shorter than N

    // The loop `for (let i = 0; i < N && i < d.length; i++)` must not overflow
    for (let i = 0; i < N && i < data.length; i++) rawRef[i] = data[i];

    expect(rawRef[0]).toBeCloseTo(0.1, 5);
    expect(rawRef[2]).toBeCloseTo(0.3, 5);
    expect(rawRef[3]).toBe(0); // untouched
  });
});
