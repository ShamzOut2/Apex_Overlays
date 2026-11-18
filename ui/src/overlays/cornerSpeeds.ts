// Corner speeds for the driver inputs: the top speed going into each corner (stored when you
// brake) and the lowest speed through it (stored when you are back on the throttle).

export interface CornerOpts {
  /** Brake pressure (0..1) that marks the start of a corner. */
  brakeOn: number;
  /** Throttle (0..1), with the brake released, that marks the way out. */
  throttleOn: number;
}

export interface CornerSpeeds {
  /** Top speed before the last corner (m/s), stored when the brake went on. */
  max: number | null;
  /** Lowest speed through the last corner (m/s), stored when the throttle went back on. */
  min: number | null;
  /** In a corner now: braked, not yet back on the throttle (the minimum is still falling). */
  inCorner: boolean;
}

/** Below this (m/s, about 18 km/h) the car is crawling (pit lane, a spin, the grid): no corner. */
const CRAWL = 5;

export function cornerSpeeds(o: CornerOpts) {
  let peak = 0;
  let low = Infinity;
  let inCorner = false;
  const out: CornerSpeeds = { max: null, min: null, inCorner: false };
  return {
    /** One telemetry sample: speed in m/s, pedals 0..1. */
    push(speed: number, throttle: number, brake: number, onTrack = true): CornerSpeeds {
      if (!onTrack || speed < CRAWL) {
        // Off track, in the garage or stopped: start again from the next run.
        peak = Math.max(0, speed);
        low = Infinity;
        inCorner = false;
        out.inCorner = false;
        return out;
      }
      if (!inCorner) {
        peak = Math.max(peak, speed);
        if (brake >= o.brakeOn) {
          // Braking for a corner: the speed carried in is the top speed since the last one.
          inCorner = true;
          out.max = peak;
          low = speed;
        }
      } else {
        low = Math.min(low, speed);
        if (throttle >= o.throttleOn && brake < o.brakeOn) {
          // Back on the throttle: the lowest speed is the corner's minimum.
          inCorner = false;
          out.min = low;
          peak = speed;
        }
      }
      out.inCorner = inCorner;
      return out;
    },
  };
}
