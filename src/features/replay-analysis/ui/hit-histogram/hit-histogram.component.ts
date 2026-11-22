import { ChangeDetectionStrategy, Component, computed, input, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Judgement, ReplayAnalysis } from '@entities/local-scores';
import { TimingView } from '../../stores/replay-analysis.store';
import { barPath, limitOf, niceStep, samplesOf, signed } from '../charts/charts';
import { hostWidth } from '../charts/width';

const HEIGHT = 220;
const MARGIN = { top: 22, right: 8, bottom: 24, left: 40 };
/** Width of a bin (ms). */
const BIN = 2;

/** The windows drawn behind the bars, from the inside out. */
const WINDOWS: Judgement[] = ['max', 'n300', 'n200', 'n100', 'n50'];

interface Bin {
    from: number;
    count: number;
    path: string;
    side: 'early' | 'late' | 'zero';
    x: number;
}

/** How many hits (or releases) came at each offset, against the judgement windows. */
@Component({
    selector: 'app-hit-histogram',
    imports: [TranslocoPipe],
    templateUrl: './hit-histogram.component.html',
    styleUrl: './hit-histogram.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HitHistogramComponent {
    readonly analysis = input.required<ReplayAnalysis>();
    readonly view = input.required<TimingView>();

    readonly width = hostWidth();
    readonly height = HEIGHT;
    readonly margin = MARGIN;
    readonly bin = BIN;
    readonly signed = signed;

    readonly hovered = signal<Bin | null>(null);

    readonly chart = computed(() => {
        const analysis = this.analysis();
        const view = this.view();
        const limit = limitOf(analysis);
        const reach = Math.ceil(limit / BIN) * BIN + BIN;

        const counts = new Map<number, number>();
        let sum = 0;
        let n = 0;
        for (const { offset } of samplesOf(analysis.stats.notes, view, limit)) {
            // Bins centred on whole steps, so the one around zero is neither early nor late.
            const from = Math.floor((offset + BIN / 2) / BIN) * BIN - BIN / 2;
            counts.set(from, (counts.get(from) ?? 0) + 1);
            sum += offset;
            n++;
        }
        const highest = Math.max(1, ...counts.values());

        const width = this.width();
        const plotWidth = width - MARGIN.left - MARGIN.right;
        const plotHeight = HEIGHT - MARGIN.top - MARGIN.bottom;
        const x = (ms: number) => MARGIN.left + ((ms + reach) / (2 * reach)) * plotWidth;
        const y = (count: number) => MARGIN.top + plotHeight - (count / highest) * plotHeight;
        const base = y(0);
        const binWidth = x(BIN) - x(0);
        // A 2px surface gap between bars, unless they are too thin for it.
        const gap = binWidth > 4 ? 1 : 0;

        const bins: Bin[] = [...counts.entries()].map(([from, count]) => ({
            from,
            count,
            path: barPath(x(from) + gap, binWidth - 2 * gap, base, y(count), Math.min(4, binWidth / 2)),
            side: from + BIN <= 0 ? 'early' : from >= 0 ? 'late' : 'zero',
            x: x(from + BIN / 2),
        }));

        // The windows as bands, alternating in shade; labels for those wide enough.
        const windows = analysis.windows;
        const bands = WINDOWS.map((judgement, i) => {
            const inner = i === 0 ? 0 : windows[WINDOWS[i - 1]];
            const outer = Math.min(windows[judgement], reach);
            return {
                judgement,
                alt: i % 2 === 1,
                rects: [
                    { x: x(-outer), width: x(-inner) - x(-outer) },
                    { x: x(inner), width: x(outer) - x(inner) },
                ],
                labelX: x((inner + outer) / 2),
                labelled: x(outer) - x(inner) > 18,
            };
        });

        const step = niceStep(2 * reach, Math.max(4, Math.floor(width / 70)));
        const ticks: { x: number; label: string }[] = [];
        for (let ms = -Math.floor(reach / step) * step; ms <= reach; ms += step) {
            ticks.push({ x: x(ms), label: signed(ms).replace('.0', '') });
        }

        const mean = n > 0 ? sum / n : null;
        return { bins, bands, ticks, base, zeroX: x(0), mean, meanX: mean === null ? 0 : x(mean), n };
    });

    readonly tooltip = computed(() => {
        const bin = this.hovered();
        if (!bin) return null;
        return { bin, left: bin.x, top: MARGIN.top };
    });
}
