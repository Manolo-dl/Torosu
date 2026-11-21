import { afterNextRender, ChangeDetectionStrategy, Component, DestroyRef, effect, ElementRef, inject, viewChild } from '@angular/core';
import { OsuAudioService, OsuReaderStore } from '@entities/osu-reader';
import { AppearanceStore } from '@shared/stores';

const BARS = 28;
/** How fast the bars rise to a level and fall from it, per second. */
const RISE = 28;
const FALL = 7;
/** While nothing plays and the bars rest, how often to look again. */
const IDLE_MS = 300;

/**
 * Bars in the title bar following the music osu! is playing, mirrored about
 * their middle, in both accents (`OsuAudioService` gives their levels).
 * Hidden while osu! is not open. Lets the pointer through, so the window
 * is still dragged from there.
 */
@Component({
    selector: 'app-visualizer',
    template: '<canvas #canvas></canvas>',
    styles: `
        :host {
            display: block;
            inline-size: 9.5rem;
            block-size: 1.75rem;
            pointer-events: none;
            opacity: 0;
            transform: scaleY(0.4);
            transition:
                opacity var(--duration-3) var(--ease-out),
                transform var(--duration-3) var(--ease-out);
        }

        :host(.shown) {
            opacity: 1;
            transform: none;
        }

        canvas {
            display: block;
            inline-size: 100%;
            block-size: 100%;
        }
    `,
    host: { 'aria-hidden': 'true', '[class.shown]': 'reader.connected()' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class VisualizerComponent {
    readonly reader = inject(OsuReaderStore);
    private readonly audio = inject(OsuAudioService);
    private readonly appearance = inject(AppearanceStore);
    private readonly canvas = viewChild.required<ElementRef<HTMLCanvasElement>>('canvas');
    private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

    private readonly shown = new Float32Array(BARS);
    private colors: [string, string] = ['#f6a', '#a6f'];
    private frame = 0;
    private idle: ReturnType<typeof setTimeout> | undefined;
    private last = 0;
    private width = 0;
    private height = 0;

    constructor() {
        const destroyRef = inject(DestroyRef);
        afterNextRender(() => {
            const resize = new ResizeObserver(() => this.resize());
            resize.observe(this.host.nativeElement);
            const onVisibility = () => this.wake();
            document.addEventListener('visibilitychange', onVisibility);
            destroyRef.onDestroy(() => {
                resize.disconnect();
                document.removeEventListener('visibilitychange', onVisibility);
                cancelAnimationFrame(this.frame);
                clearTimeout(this.idle);
            });
            this.wake();
        });

        // The accents, again when the palette or the theme changes.
        effect(() => {
            this.appearance.palette();
            this.appearance.dark();
            requestAnimationFrame(() => this.readColors());
        });
        // Waking when osu! opens.
        effect(() => {
            if (this.reader.connected()) this.wake();
        });
    }

    private readColors() {
        const style = getComputedStyle(this.host.nativeElement);
        const probe = document.createElement('span');
        this.host.nativeElement.append(probe);
        probe.style.color = 'var(--accent)';
        const accent = getComputedStyle(probe).color;
        probe.style.color = 'var(--accent-2)';
        const accent2 = getComputedStyle(probe).color;
        probe.remove();
        this.colors = [accent || style.color, accent2 || style.color];
    }

    private resize() {
        const canvas = this.canvas().nativeElement;
        const box = this.host.nativeElement.getBoundingClientRect();
        const ratio = Math.min(devicePixelRatio || 1, 2);
        this.width = box.width;
        this.height = box.height;
        canvas.width = Math.round(box.width * ratio);
        canvas.height = Math.round(box.height * ratio);
        canvas.getContext('2d')?.setTransform(ratio, 0, 0, ratio, 0, 0);
        this.draw();
    }

    /** Draws every frame while something plays or the bars still fall; then looks now and then. */
    private wake() {
        cancelAnimationFrame(this.frame);
        clearTimeout(this.idle);
        if (document.hidden || !this.reader.connected()) return;
        this.last = performance.now();
        this.frame = requestAnimationFrame(now => this.tick(now));
    }

    private tick(now: number) {
        const seconds = Math.min((now - this.last) / 1000, 0.1);
        this.last = now;
        const still = this.appearance.reducedMotion();
        const levels = this.audio.levels(BARS, now);
        let resting = true;
        for (let i = 0; i < BARS; i++) {
            const target = still ? Math.min(levels[i], 0.35) : levels[i];
            const speed = target > this.shown[i] ? RISE : FALL;
            this.shown[i] += (target - this.shown[i]) * Math.min(1, speed * seconds);
            if (this.shown[i] > 0.01) resting = false;
        }
        this.draw();

        if (resting && !this.audio.playing(now)) {
            this.idle = setTimeout(() => this.wake(), IDLE_MS);
        } else {
            this.frame = requestAnimationFrame(next => this.tick(next));
        }
    }

    private draw() {
        const context = this.canvas().nativeElement.getContext('2d');
        if (!context || !this.width) return;
        const { width, height } = this;
        context.clearRect(0, 0, width, height);

        const gap = 2;
        const bar = (width - gap * (BARS - 1)) / BARS;
        const gradient = context.createLinearGradient(0, 0, width, 0);
        gradient.addColorStop(0, this.colors[0]);
        gradient.addColorStop(1, this.colors[1]);
        context.fillStyle = gradient;

        for (let i = 0; i < BARS; i++) {
            // At rest, a dot; playing, up and down from the middle.
            const size = Math.max(bar, this.shown[i] * height);
            const x = i * (bar + gap);
            const y = (height - size) / 2;
            context.globalAlpha = 0.45 + this.shown[i] * 0.55;
            context.beginPath();
            context.roundRect(x, y, bar, size, bar / 2);
            context.fill();
        }
        context.globalAlpha = 1;
    }
}
