import { ChangeDetectionStrategy, Component, ElementRef, computed, effect, inject, input, signal, viewChild } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ReplayAnalysis } from '@entities/local-scores';
import { TUI_DARK_MODE } from '@taiga-ui/core';
import { TimingView } from '../../stores/replay-analysis.store';
import { clock, limitOf, niceStep, offsetOf, signed } from '../charts/charts';

const WIDTH = 720;
const HEIGHT = 240;
const MARGIN = { top: 16, right: 8, bottom: 24, left: 40 };
/** The canvas draws at this many pixels per unit of the chart, sharp on HiDPI screens. */
const RESOLUTION = 2;
/** Offsets each point of the moving mean averages. */
const MOVING = 50;

interface Point {
    time: number;
    offset: number;
}

/** Every offset along the song, its moving mean, and where the game counted misses. */
@Component({
    selector: 'app-timeline',
    imports: [TranslocoPipe],
    templateUrl: './timeline.component.html',
    styleUrl: './timeline.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TimelineComponent {
    readonly analysis = input.required<ReplayAnalysis>();
    readonly view = input.required<TimingView>();

    readonly width = WIDTH;
    readonly height = HEIGHT;
    readonly margin = MARGIN;
    readonly moving = MOVING;
    readonly resolution = RESOLUTION;
    readonly clock = clock;
    readonly signed = signed;

    private readonly host = inject(ElementRef<HTMLElement>);
    private readonly darkMode = inject(TUI_DARK_MODE);
    private readonly canvas = viewChild.required<ElementRef<HTMLCanvasElement>>('canvas');

    /** Where the pointer is, in chart units. */
    readonly pointer = signal<number | null>(null);

    readonly points = computed((): Point[] => {
        const analysis = this.analysis();
        const limit = limitOf(analysis);
        const view = this.view();
        const points: Point[] = [];
        for (const note of analysis.stats.notes) {
            const offset = offsetOf(note, view, limit);
            if (offset !== null) points.push({ time: note.time, offset });
        }
        return points;
    });

    readonly chart = computed(() => {
        const analysis = this.analysis();
        const points = this.points();
        const reach = Math.ceil(limitOf(analysis) / 10) * 10;
        const end = Math.max(1000, ...analysis.stats.notes.map(n => n.time));

        const plotWidth = WIDTH - MARGIN.left - MARGIN.right;
        const plotHeight = HEIGHT - MARGIN.top - MARGIN.bottom;
        const x = (time: number) => MARGIN.left + (time / end) * plotWidth;
        const y = (ms: number) => MARGIN.top + ((reach - ms) / (2 * reach)) * plotHeight;

        // The moving mean, centred on each offset, at most ~400 points along the path.
        const sums = [0];
        for (const p of points) sums.push(sums[sums.length - 1] + p.offset);
        const every = Math.max(1, Math.floor(points.length / 400));
        const mean: { time: number; value: number }[] = [];
        for (let i = 0; i < points.length; i += every) {
            const from = Math.max(0, i - MOVING / 2);
            const to = Math.min(points.length, i + MOVING / 2);
            mean.push({ time: points[i].time, value: (sums[to] - sums[from]) / (to - from) });
        }
        const meanPath = mean.map((m, i) => `${i ? 'L' : 'M'}${x(m.time).toFixed(1)},${y(m.value).toFixed(1)}`).join(' ');

        const yStep = niceStep(2 * reach, 6);
        const yTicks: { y: number; label: string }[] = [];
        for (let ms = -Math.floor(reach / yStep) * yStep; ms <= reach; ms += yStep) {
            yTicks.push({ y: y(ms), label: signed(ms).replace('.0', '') });
        }
        const xStep = niceStep(end / 1000, 8) * 1000;
        const xTicks: { x: number; label: string }[] = [];
        for (let t = 0; t <= end; t += xStep) xTicks.push({ x: x(t), label: clock(t) });

        const misses = analysis.gameMisses.map(time => x(time));
        return { x, y, end, meanPath, mean, yTicks, xTicks, misses, zero: y(0), plotWidth };
    });

    /** The moving mean and the misses around the pointer. */
    readonly tooltip = computed(() => {
        const at = this.pointer();
        const c = this.chart();
        if (at === null || c.mean.length === 0) return null;
        const time = ((at - MARGIN.left) / c.plotWidth) * c.end;
        let nearest = c.mean[0];
        for (const m of c.mean) if (Math.abs(m.time - time) < Math.abs(nearest.time - time)) nearest = m;
        const x = c.x(nearest.time);
        return { x, time: nearest.time, value: nearest.value, left: (x / WIDTH) * 100, top: (MARGIN.top / HEIGHT) * 100 };
    });

    constructor() {
        effect(() => {
            const points = this.points();
            const chart = this.chart();
            this.darkMode();
            // After the theme's colours are applied.
            requestAnimationFrame(() => this.draw(points, chart.x, chart.y));
        });
    }

    move(event: MouseEvent) {
        const svg = event.currentTarget as SVGElement;
        const box = svg.getBoundingClientRect();
        const at = ((event.clientX - box.left) / box.width) * WIDTH;
        this.pointer.set(at >= MARGIN.left && at <= WIDTH - MARGIN.right ? at : null);
    }

    private draw(points: Point[], x: (time: number) => number, y: (ms: number) => number) {
        const canvas = this.canvas().nativeElement;
        const context = canvas.getContext('2d');
        if (!context) return;
        const style = getComputedStyle(this.host.nativeElement);
        const early = style.getPropertyValue('--viz-early').trim();
        const late = style.getPropertyValue('--viz-late').trim();

        context.setTransform(RESOLUTION, 0, 0, RESOLUTION, 0, 0);
        context.clearRect(0, 0, WIDTH, HEIGHT);
        context.globalAlpha = 0.45;
        for (const p of points) {
            context.fillStyle = p.offset < 0 ? early : late;
            context.fillRect(x(p.time) - 0.75, y(p.offset) - 0.75, 1.5, 1.5);
        }
        context.globalAlpha = 1;
    }
}
