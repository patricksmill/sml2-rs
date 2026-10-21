//! Band-limited step synthesis for the APU mixer (the Blip_Buffer approach).
//!
//! Every channel output is a stair of levels. Instead of sampling it, each
//! level change is drawn as a band-limited step at its exact sub-sample time:
//! the step's derivative, a Kaiser-windowed sinc, is added to a delta buffer,
//! and reading the buffer integrates it back into levels. Harmonics above the
//! cutoff never reach the output, so they cannot alias back below Nyquist.
//!
//! The kernel is integer and every phase sums to exactly [`UNIT`], so a step
//! integrates back to precisely its delta and the output never drifts.

use std::sync::OnceLock;

/// Taps per step. The output lags the input by about `WIDTH / 2` samples.
pub const WIDTH: usize = 32;
/// Sub-sample positions a step can start at.
const PHASES: usize = 256;
/// Each kernel phase sums to this.
const KERNEL_UNIT: i64 = 1 << 15;
/// Weight steps between two neighbouring phases.
const INTERP: i64 = 1 << 8;
/// Fixed-point value of one level unit after integration.
pub const UNIT: i64 = KERNEL_UNIT * INTERP;
/// The sinc's -6 dB point as a fraction of the sample rate (19.8 kHz at
/// 44.1 kHz). With the window below the stopband starts near 0.53 fs, so the
/// only content folding back lands above 20 kHz.
const CUTOFF: f64 = 0.45;
/// Kaiser window shape: about 80 dB of stopband attenuation.
const KAISER_BETA: f64 = 8.0;

type Kernel = [[i32; WIDTH]; PHASES + 1];

/// Phase `p` holds the step derivative for a step `p / PHASES` of a sample
/// after the slot it starts in; phase `PHASES` is phase 0 one slot later, so
/// every phase has a right-hand neighbour to interpolate towards.
fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Box<Kernel>> = OnceLock::new();
    KERNEL.get_or_init(|| {
        let mut table = Box::new([[0i32; WIDTH]; PHASES + 1]);
        let half = (WIDTH / 2) as f64;
        for (phase, taps) in table.iter_mut().enumerate() {
            let frac = phase as f64 / PHASES as f64;
            let mut shape = [0f64; WIDTH];
            for (k, tap) in shape.iter_mut().enumerate() {
                let x = k as f64 + 1.0 - half - frac;
                *tap = sinc(2.0 * CUTOFF * x) * kaiser(x / half);
            }
            let sum: f64 = shape.iter().sum();
            let mut total = 0i64;
            for (tap, &s) in taps.iter_mut().zip(&shape) {
                *tap = (s / sum * KERNEL_UNIT as f64).round() as i32;
                total += *tap as i64;
            }
            // Fold the rounding residue into the largest tap: each phase then
            // sums to exactly KERNEL_UNIT and a step integrates to exactly its
            // delta.
            let peak = (0..WIDTH).max_by_key(|&k| taps[k].abs()).unwrap_or(0);
            taps[peak] += (KERNEL_UNIT - total) as i32;
        }
        table
    })
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        1.0
    } else {
        let px = std::f64::consts::PI * x;
        px.sin() / px
    }
}

/// Kaiser window over `u` in `[-1, 1]`.
fn kaiser(u: f64) -> f64 {
    if u.abs() >= 1.0 {
        return 0.0;
    }
    bessel_i0(KAISER_BETA * (1.0 - u * u).sqrt()) / bessel_i0(KAISER_BETA)
}

/// Zeroth-order modified Bessel function of the first kind (power series).
fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let half = x / 2.0;
    for k in 1..64 {
        term *= half / k as f64;
        let t = term * term;
        sum += t;
        if t < sum * 1e-17 {
            break;
        }
    }
    sum
}

/// A stereo delta buffer and its integrators.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StereoBlip {
    /// Pending deltas; slot 0 is the next sample `read` produces. Slots past
    /// the last read carry the tails of steps into the next block.
    deltas: Vec<[i64; 2]>,
    level: [i64; 2],
}

impl StereoBlip {
    /// Makes room for a block of `samples` plus the kernel tail.
    pub fn begin(&mut self, samples: usize) {
        let len = samples + WIDTH + 2;
        if self.deltas.len() < len {
            self.deltas.resize(len, [0, 0]);
        }
    }

    /// Adds a step of `delta` level units starting at `time` samples into the
    /// block. `time` may be as early as -1 (the slot before the block).
    pub fn add_step(&mut self, time: f64, delta: [i32; 2]) {
        // Interpolate linearly between the two nearest phases. The weights
        // sum to INTERP, so the step still integrates to exactly delta × UNIT.
        let pos = ((time + 1.0).max(0.0) * (PHASES as i64 * INTERP) as f64) as i64;
        let slot = (pos / (PHASES as i64 * INTERP)) as usize;
        let phase = ((pos / INTERP) % PHASES as i64) as usize;
        let w = pos % INTERP;
        let kernel = kernel();
        let (lo, hi) = (&kernel[phase], &kernel[phase + 1]);
        let [dl, dr] = [delta[0] as i64, delta[1] as i64];
        for (k, d) in self.deltas[slot..slot + WIDTH].iter_mut().enumerate() {
            let tap = lo[k] as i64 * (INTERP - w) + hi[k] as i64 * w;
            d[0] += dl * tap;
            d[1] += dr * tap;
        }
    }

    /// Integrates the first `samples` slots, passing each level (in units of
    /// [`UNIT`]) to `emit`, and moves the remaining tail to the front.
    pub fn read(&mut self, samples: usize, mut emit: impl FnMut([i64; 2])) {
        for d in &self.deltas[..samples] {
            self.level[0] += d[0];
            self.level[1] += d[1];
            emit(self.level);
        }
        let len = self.deltas.len();
        self.deltas.copy_within(samples..len, 0);
        self.deltas[len - samples..].fill([0, 0]);
    }
}
