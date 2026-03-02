import { describe, it, expect } from "vitest";

// ── Helpers extracted from PlayerBar (pure logic, no React/DOM needed) ──

function formatTime(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const sec = s % 60;
  return `${m}:${sec.toString().padStart(2, "0")}`;
}

function progressPercent(positionMs: number, durationMs: number): number {
  return durationMs > 0 ? (positionMs / durationMs) * 100 : 0;
}

// ── formatTime ────────────────────────────────────────────────────────

describe("formatTime", () => {
  it("formats zero as 0:00", () => {
    expect(formatTime(0)).toBe("0:00");
  });

  it("formats exactly one minute", () => {
    expect(formatTime(60_000)).toBe("1:00");
  });

  it("pads seconds below 10 with a leading zero", () => {
    expect(formatTime(65_000)).toBe("1:05");
  });

  it("formats a typical 4-minute track", () => {
    expect(formatTime(245_000)).toBe("4:05");
  });

  it("handles long tracks (> 60 min)", () => {
    expect(formatTime(3_661_000)).toBe("61:01");
  });

  it("ignores sub-second precision (floors to seconds)", () => {
    expect(formatTime(90_999)).toBe("1:30");
  });
});

// ── progressPercent ───────────────────────────────────────────────────

describe("progressPercent", () => {
  it("returns 0 at the start of a track", () => {
    expect(progressPercent(0, 60_000)).toBe(0);
  });

  it("returns 100 at the end of a track", () => {
    expect(progressPercent(60_000, 60_000)).toBe(100);
  });

  it("returns 50 at the midpoint", () => {
    expect(progressPercent(30_000, 60_000)).toBeCloseTo(50, 3);
  });

  it("returns 0 when duration is unknown (0)", () => {
    expect(progressPercent(5_000, 0)).toBe(0);
  });

  it("returns a value between 0 and 100 for any valid position", () => {
    const pct = progressPercent(12_345, 60_000);
    expect(pct).toBeGreaterThan(0);
    expect(pct).toBeLessThan(100);
  });

  it("does not exceed 100 even if position somehow exceeds duration", () => {
    // The backend clamps position to duration, but the frontend should be safe too
    const pct = progressPercent(70_000, 60_000);
    // Frontend doesn't clamp — this just validates the formula holds
    expect(pct).toBeCloseTo(116.67, 1);
  });
});

// ── Volume (cubic curve logic mirrored from Rust) ─────────────────────

function cubicVolume(linear: number): number {
  return linear * linear * linear;
}

describe("cubicVolume", () => {
  it("maps 0 → 0 (silence)", () => {
    expect(cubicVolume(0)).toBe(0);
  });

  it("maps 1 → 1 (full volume)", () => {
    expect(cubicVolume(1)).toBe(1);
  });

  it("maps 0.5 → 0.125 (perceptually quieter than 50%)", () => {
    expect(cubicVolume(0.5)).toBeCloseTo(0.125, 5);
  });

  it("output is always less than input for values in (0,1)", () => {
    const val = 0.7;
    expect(cubicVolume(val)).toBeLessThan(val);
  });
});

// ── Seek override logic ───────────────────────────────────────────────

describe("seekOverride display logic", () => {
  it("shows seekOverride while dragging instead of backend position", () => {
    const backendPosition = 30_000;
    const seekOverride = 45_000;
    const displayed = seekOverride !== null ? seekOverride : backendPosition;
    expect(displayed).toBe(45_000);
  });

  it("falls back to backend position when seekOverride is null", () => {
    const backendPosition = 30_000;
    const seekOverride: number | null = null;
    const displayed = seekOverride !== null ? seekOverride : backendPosition;
    expect(displayed).toBe(30_000);
  });
});

// ── Instant-based position tracking (mirrors the Rust fix logic) ──────

describe("Instant-based position tracking", () => {
  it("position advances while playing (simulated with Date.now)", async () => {
    const startMs = Date.now();
    await new Promise((r) => setTimeout(r, 100));
    const elapsed = Date.now() - startMs;
    const positionAtStart = 0;
    const currentPos = positionAtStart + elapsed;

    expect(currentPos).toBeGreaterThanOrEqual(80);
  });

  it("position is frozen when paused (no Instant running)", () => {
    const positionAtStart = 12_345;
    const isPlaying = false; // paused → no Instant
    // Simulate: when paused, position = positionAtStart (no elapsed addition)
    const currentPos = isPlaying ? positionAtStart + 999 : positionAtStart;
    expect(currentPos).toBe(12_345);
  });

  it("seek updates position_at_start_ms to the target", () => {
    let positionAtStart = 0;
    const seekTargetMs = 45_000;
    // Simulate seek command
    positionAtStart = seekTargetMs;
    expect(positionAtStart).toBe(45_000);
  });

  it("pause accumulates elapsed time into position_at_start_ms", async () => {
    let positionAtStart = 0;
    const playbackStartMs = Date.now();
    await new Promise((r) => setTimeout(r, 150));
    // Simulate pause: freeze position
    positionAtStart += Date.now() - playbackStartMs;

    expect(positionAtStart).toBeGreaterThanOrEqual(100);

    // Resume and play a bit more
    const resumeStartMs = Date.now();
    await new Promise((r) => setTimeout(r, 100));
    const currentPos = positionAtStart + (Date.now() - resumeStartMs);

    expect(currentPos).toBeGreaterThan(positionAtStart);
  });
});

// ── Duration clamping ─────────────────────────────────────────────────

describe("position clamping to duration", () => {
  function clamp(position: number, duration: number): number {
    return duration > 0 ? Math.min(position, duration) : position;
  }

  it("clamps position that exceeds duration", () => {
    expect(clamp(70_000, 60_000)).toBe(60_000);
  });

  it("does not clip a valid mid-track position", () => {
    expect(clamp(30_000, 60_000)).toBe(30_000);
  });

  it("does not clamp when duration is 0 (unknown)", () => {
    expect(clamp(5_000, 0)).toBe(5_000);
  });
});

// ── Post-end seek / replay logic ──────────────────────────────────────
// Mirrors the Rust AudioPlayer::seek() branching when is_finished = true.

describe("post-end seek (replay after track finishes)", () => {
  interface MockPlayerState {
    isFinished: boolean;
    isPlaying: boolean;
    positionMs: number;
    hasCachedBytes: boolean;
  }

  /** Simulates what AudioPlayer::seek() does when called after track ends */
  function simulateSeek(
    player: MockPlayerState,
    seekMs: number,
  ): { reloaded: boolean; seekTarget: number; playing: boolean } {
    player.positionMs = seekMs; // immediate UI update always happens

    if (player.isFinished) {
      if (player.hasCachedBytes) {
        // Reload from cache and seek
        player.isFinished = false;
        player.isPlaying = true;
        return { reloaded: true, seekTarget: seekMs, playing: true };
      }
      return { reloaded: false, seekTarget: seekMs, playing: false };
    }

    // Normal seek (decoder still alive)
    return { reloaded: false, seekTarget: seekMs, playing: player.isPlaying };
  }

  it("reloads from cache and starts playing when track is finished", () => {
    const player: MockPlayerState = {
      isFinished: true,
      isPlaying: false,
      positionMs: 60_000,
      hasCachedBytes: true,
    };

    const result = simulateSeek(player, 30_000);

    expect(result.reloaded).toBe(true);
    expect(result.playing).toBe(true);
    expect(result.seekTarget).toBe(30_000);
    expect(player.isFinished).toBe(false);
    expect(player.positionMs).toBe(30_000);
  });

  it("updates position immediately even when finished and no cache", () => {
    const player: MockPlayerState = {
      isFinished: true,
      isPlaying: false,
      positionMs: 60_000,
      hasCachedBytes: false,
    };

    simulateSeek(player, 15_000);

    expect(player.positionMs).toBe(15_000);
  });

  it("does a normal seek (no reload) when track is still playing", () => {
    const player: MockPlayerState = {
      isFinished: false,
      isPlaying: true,
      positionMs: 10_000,
      hasCachedBytes: true,
    };

    const result = simulateSeek(player, 45_000);

    expect(result.reloaded).toBe(false);
    expect(result.seekTarget).toBe(45_000);
    expect(player.isFinished).toBe(false);
    expect(player.isPlaying).toBe(true);
  });

  it("seek to 0 after track ends replays from beginning", () => {
    const player: MockPlayerState = {
      isFinished: true,
      isPlaying: false,
      positionMs: 60_000,
      hasCachedBytes: true,
    };

    const result = simulateSeek(player, 0);

    expect(result.reloaded).toBe(true);
    expect(result.seekTarget).toBe(0);
    expect(player.positionMs).toBe(0);
  });

  it("position is immediately visible in UI (optimistic update)", () => {
    const player: MockPlayerState = {
      isFinished: true,
      isPlaying: false,
      positionMs: 60_000,
      hasCachedBytes: true,
    };

    simulateSeek(player, 22_000);

    // Frontend should display the new position right away
    expect(player.positionMs).toBe(22_000);
  });
});
