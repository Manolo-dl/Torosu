import { NoteEvent, ReplayAnalysis } from '@entities/local-scores';
import { TimingView } from '../../stores/replay-analysis.store';

/** A timing of a note: when, in the song (ms), and how far off (ms). */
export interface Sample {
    time: number;
    offset: number;
}

/**
 * The timings `view` counts of `notes`, within `limit` (ms): beyond it they
 * are misses. The same ones as the backend's `Views`.
 */
export function samplesOf(notes: NoteEvent[], view: TimingView, limit: number): Sample[] {
    const samples: Sample[] = [];
    const add = (time: number, offset: number | null) => {
        if (offset !== null && Math.abs(offset) <= limit) samples.push({ time, offset });
    };
    for (const note of notes) {
        const long = note.end !== null;
        if (view === 'all' || (view === 'rice') !== long) add(note.time, note.hit);
        // A long note's release counts only if its start was pressed.
        if (view !== 'rice' && long && note.hit !== null) add(note.end!, note.release);
    }
    return samples.sort((a, b) => a.time - b.time);
}

/**
 * How far the charts reach (ms): the 50 window, beyond which a press is a miss
 * and not a timing.
 */
export function limitOf(analysis: ReplayAnalysis): number {
    return analysis.windows.n50;
}

/** A round step for an axis spanning `range`, giving about `ticks` ticks. */
export function niceStep(range: number, ticks: number): number {
    const raw = range / Math.max(1, ticks);
    const power = 10 ** Math.floor(Math.log10(raw));
    const scaled = raw / power;
    const nice = scaled <= 1 ? 1 : scaled <= 2 ? 2 : scaled <= 5 ? 5 : 10;
    return nice * power;
}

/** `ms` as `m:ss`. */
export function clock(ms: number): string {
    const seconds = Math.max(0, Math.round(ms / 1000));
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}

/** `value` with its sign, to one decimal: `+3.2`, `-0.5`. */
export function signed(value: number): string {
    return `${value > 0 ? '+' : ''}${value.toFixed(1)}`;
}

/**
 * A bar from `y0` (the baseline) to `y1`, `width` wide from `x`, with its end
 * away from the baseline rounded by `radius`.
 */
export function barPath(x: number, width: number, y0: number, y1: number, radius = 4): string {
    const height = Math.abs(y1 - y0);
    const r = Math.min(radius, width / 2, height);
    const up = y1 < y0;
    const end = y1;
    const dir = up ? 1 : -1;
    // From the baseline left, up to the rounded end, across, and back down.
    return [
        `M${x},${y0}`,
        `L${x},${end + dir * r}`,
        `Q${x},${end} ${x + r},${end}`,
        `L${x + width - r},${end}`,
        `Q${x + width},${end} ${x + width},${end + dir * r}`,
        `L${x + width},${y0}`,
        'Z',
    ].join(' ');
}
