import { afterNextRender, ChangeDetectionStrategy, Component, DestroyRef, effect, ElementRef, inject, viewChild } from '@angular/core';
import { MUSIC_BANDS, Music, MusicService, OsuReaderStore } from '@entities/osu-reader';
import { AppearanceStore } from '@shared/stores';

/**
 * The music osu! is playing across the whole title bar, behind its contents:
 * bars rising from its bottom edge, the bass in the middle and the highs out
 * to the sides (where the buttons are, the music is quietest), fading out
 * upwards so what is over them stays readable, brighter on each bass hit.
 * `MusicService` gives their levels. Hidden while osu! is not open. Lets the
 * pointer through: the window still drags from the title bar.
 */
@Component({
    selector: 'app-visualizer',
    template: '<canvas #canvas></canvas>',
    styles: `
        :host {
            position: absolute;
            inset: 0;
            pointer-events: none;
            opacity: 0;
            transition: opacity var(--duration-4) var(--ease-out);
        }

        :host(.shown) {
            opacity: 1;
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

        // About 3px bars, no more than 180 of them: wider on wide windows.
        const gap = 2;
        const count = Math.min(180, Math.floor(width / 5));
        const bar = (width - gap * (count - 1)) / count;
        const middle = (count - 1) / 2;

        // The second accent out to the sides, the first in the middle.
        const gradient = context.createLinearGradient(0, 0, width, 0);
        gradient.addColorStop(0, this.colors[1]);
        gradient.addColorStop(0.5, this.colors[0]);
        gradient.addColorStop(1, this.colors[1]);
        context.fillStyle = gradient;
        context.globalAlpha = Math.min(0.85, 0.45 + beat * 0.3);

        for (let i = 0; i < count; i++) {
            // How far from the middle, 0 to 1: which band, between two of them.
            const at = (Math.abs(i - middle) / middle) * (MUSIC_BANDS - 1);
            const low = Math.floor(at);
            const high = Math.min(MUSIC_BANDS - 1, low + 1);
            const level = Math.min(reach, levels[low] + (levels[high] - levels[low]) * (at - low));
            const size = Math.max(1.5, level * height * 0.9);
            context.fillRect(i * (bar + gap), height - size, bar, size);
        }

        // Fading out upwards: what is over the bars stays readable.
        context.globalAlpha = 1;
        context.globalCompositeOperation = 'destination-in';
        const fade = context.createLinearGradient(0, height, 0, 0);
        fade.addColorStop(0, 'rgba(0, 0, 0, 1)');
        fade.addColorStop(1, 'rgba(0, 0, 0, 0.15)');
        context.fillStyle = fade;
        context.fillRect(0, 0, width, height);
        context.globalCompositeOperation = 'source-over';
        return false;
    }
}
