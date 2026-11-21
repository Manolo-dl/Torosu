import { afterNextRender, ChangeDetectionStrategy, Component, DestroyRef, effect, ElementRef, inject, viewChild } from '@angular/core';
import { MUSIC_BANDS, Music, MusicService, OsuReaderStore } from '@entities/osu-reader';
import { AppearanceStore } from '@shared/stores';

/**
 * Bars in the title bar following the music osu! is playing, mirrored about
 * their middle, in both accents and brighter on each bass hit
 * (`MusicService` gives their levels). Hidden while osu! is not open. Lets
 * the pointer through, so the window is still dragged from there.
 */
@Component({
    selector: 'app-visualizer',
    template: '<canvas #canvas></canvas>',
    styles: `
        :host {
            display: block;
            inline-size: clamp(9rem, 22vw, 17rem);
            block-size: 2.4rem;
            pointer-events: none;
            opacity: 0;
            transform: scaleY(0.3);
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
    private readonly music = inject(MusicService);
    private readonly appearance = inject(AppearanceStore);
    private readonly canvas = viewChild.required<ElementRef<HTMLCanvasElement>>('canvas');
    private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

    private colors: [string, string] = ['#f6a', '#a6f'];
    private width = 0;
    private height = 0;
    private last: Music | null = null;

    constructor() {
        const destroyRef = inject(DestroyRef);
        afterNextRender(() => {
            const resize = new ResizeObserver(() => this.resize());
            resize.observe(this.host.nativeElement);
            const stop = this.music.listen(music => this.draw(music));
            destroyRef.onDestroy(() => {
                resize.disconnect();
                stop();
            });
        });

        // The accents, again when the palette or the theme changes.
        effect(() => {
            this.appearance.palette();
            this.appearance.dark();
            requestAnimationFrame(() => this.readColors());
        });
        // Waking when osu! opens; 30 frames a second at the low quality.
        effect(() => {
            this.music.halfRate = this.appearance.quality() === 'low';
            if (this.reader.connected()) this.music.wake();
        });
    }

    private readColors() {
        const probe = document.createElement('span');
        this.host.nativeElement.append(probe);
        probe.style.color = 'var(--accent)';
        const accent = getComputedStyle(probe).color;
        probe.style.color = 'var(--accent-2)';
        const accent2 = getComputedStyle(probe).color;
        probe.remove();
        this.colors = [accent, accent2];
        if (this.last) this.draw(this.last);
    }

    private resize() {
        const canvas = this.canvas().nativeElement;
        const box = this.host.nativeElement.getBoundingClientRect();
        const ratio = this.appearance.quality() === 'low' ? 1 : Math.min(devicePixelRatio || 1, 2);
        this.width = box.width;
        this.height = box.height;
        canvas.width = Math.round(box.width * ratio);
        canvas.height = Math.round(box.height * ratio);
        canvas.getContext('2d')?.setTransform(ratio, 0, 0, ratio, 0, 0);
        if (this.last) this.draw(this.last);
    }

    /** Returns false: the bars rest with the music, which `MusicService` knows. */
    private draw(music: Music): boolean {
        this.last = music;
        const context = this.canvas().nativeElement.getContext('2d');
        if (!context || !this.width) return false;
        const { width, height } = this;
        const { levels, beat } = music;
        // Reduced motion: the bars move, but only a little.
        const reach = this.appearance.reducedMotion() ? 0.35 : 1;
        context.clearRect(0, 0, width, height);

        const gap = 2;
        const bar = (width - gap * (MUSIC_BANDS - 1)) / MUSIC_BANDS;
        const gradient = context.createLinearGradient(0, 0, width, 0);
        gradient.addColorStop(0, this.colors[0]);
        gradient.addColorStop(1, this.colors[1]);
        context.fillStyle = gradient;

        for (let i = 0; i < MUSIC_BANDS; i++) {
            const level = Math.min(levels[i], reach);
            // At rest, a dot; playing, up and down from the middle.
            const size = Math.max(bar, level * height);
            const x = i * (bar + gap);
            const y = (height - size) / 2;
            context.globalAlpha = Math.min(1, 0.4 + level * 0.5 + beat * 0.3);
            context.beginPath();
            context.roundRect(x, y, bar, size, bar / 2);
            context.fill();
        }
        context.globalAlpha = 1;
        return false;
    }
}
