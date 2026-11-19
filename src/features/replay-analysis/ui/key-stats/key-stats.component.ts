import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, input, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { KeyStats, ReplayAnalysis, Timing } from '@entities/local-scores';
import { TimingView } from '../../stores/replay-analysis.store';
import { barPath, niceStep, signed } from '../charts/charts';
import { hostWidth } from '../charts/width';

const HEIGHT = 200;
const MARGIN = { top: 12, right: 8, bottom: 24, left: 40 };

interface Bar {
    key: number;
    stats: KeyStats;
    timing: Timing;
    path: string | null;
    early: boolean;
    /** Middle of its band, for the label and the tooltip. */
    center: number;
    band: { x: number; width: number };
}

/** Mean offset of each key as a bar above (late) or below (early) zero, and a table of every number. */
@Component({
    selector: 'app-key-stats',
    imports: [DecimalPipe, TranslocoPipe],
    templateUrl: './key-stats.component.html',
    styleUrl: './key-stats.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class KeyStatsComponent {
    readonly analysis = input.required<ReplayAnalysis>();
    readonly view = input.required<TimingView>();

    readonly width = hostWidth();
    readonly height = HEIGHT;
    readonly margin = MARGIN;
    readonly signed = signed;

    readonly hovered = signal<number | null>(null);

    readonly chart = computed(() => {
        const keys = this.analysis().stats.keys;
        const view = this.view();
        const timingOf = (k: KeyStats) => k.timing[view];
        const means = keys.filter(k => timingOf(k).count > 0).map(k => Math.abs(timingOf(k).mean));
        // Symmetric around zero, so early and late bars compare.
        const step = niceStep(Math.max(5, ...means), 2);
        const reach = Math.ceil(Math.max(5, ...means) / step) * step;
        const plotHeight = HEIGHT - MARGIN.top - MARGIN.bottom;
        const y = (ms: number) => MARGIN.top + ((reach - ms) / (2 * reach)) * plotHeight;
        const zero = y(0);

        const plotWidth = this.width() - MARGIN.left - MARGIN.right;
        const bandWidth = plotWidth / keys.length;
        const barWidth = Math.min(48, bandWidth * 0.6);

        const bars: Bar[] = keys.map((stats, i) => {
            const timing = timingOf(stats);
            const x = MARGIN.left + i * bandWidth;
            const center = x + bandWidth / 2;
            // A mean that rounds to nothing still shows a sliver at zero.
            const end = Math.abs(y(timing.mean) - zero) < 1 ? zero + (timing.mean < 0 ? 1 : -1) : y(timing.mean);
            return {
                key: stats.key,
                stats,
                timing,
                path: timing.count > 0 ? barPath(center - barWidth / 2, barWidth, zero, end) : null,
                early: timing.mean < 0,
                center,
                band: { x, width: bandWidth },
            };
        });

        const ticks: { y: number; label: string }[] = [];
        for (let ms = -reach; ms <= reach; ms += step) {
            ticks.push({ y: y(ms), label: signed(ms).replace('.0', '') });
        }
        return { bars, ticks, zero };
    });

    /** Whether the table tells the long notes let go early or held too long. */
    readonly showReleases = computed(() => this.view() !== 'rice' && this.analysis().stats.timing.ln.count > 0);

    readonly tooltip = computed(() => {
        const key = this.hovered();
        const bar = this.chart().bars.find(b => b.key === key);
        if (!bar) return null;
        return { bar, left: bar.center, top: MARGIN.top };
    });
}
