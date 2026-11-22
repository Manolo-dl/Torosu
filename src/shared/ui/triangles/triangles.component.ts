import { afterNextRender, ChangeDetectionStrategy, Component, DestroyRef, effect, ElementRef, inject, input, viewChild } from '@angular/core';
import { AppearanceStore } from '../../stores/appearance.store';
import { OsuPlayState } from '../../stores/osu-play-state';

interface Triangle {
    /** Across the box, 0 to 1. */
    x: number;
    /** Down the box, 0 to 1, of the top corner. */
    y: number;
    /** Side, in CSS pixels. */
    size: number;
    /** Which accent, and how strong. */
    second: boolean;
    alpha: number;
}

/** Side and speed of the triangles, in CSS pixels and pixels per second. */
const BASE_SIZE = 46;
const BASE_SPEED = 14;
/** One triangle per this many square pixels of the box. */
const AREA_PER_TRIANGLE = 5200;
const HEIGHT_RATIO = Math.sqrt(3) / 2;

/** A number around 0 with a standard deviation of 1 (Box–Muller). */
function gaussian(): number {
    return Math.sqrt(-2 * Math.log(1 - Math.random())) * Math.cos(2 * Math.PI * Math.random());
}

/**
 * Outlined triangles in both accents drifting up behind what is in the box,
 * as osu! draws its menus: `<app-triangles />` as the first child of a
 * positioned element. The bigger ones are closer, so faster. Still with
 * reduced motion, and paused while out of sight.
 */
@Component({
    selector: 'app-triangles',
    template: '<canvas #canvas></canvas>',
    styles: `
        :host {
            position: absolute;
            inset: 0;
            z-index: -1;
            overflow: hidden;
            pointer-events: none;
            /* From the top right corner, fading out before the text on the left. */
            -webkit-mask-image: radial-gradient(ellipse 75% 120% at 100% 0%, #000 25%, transparent 100%);
            mask-image: radial-gradient(ellipse 75% 120% at 100% 0%, #000 25%, transparent 100%);
        }

        canvas {
            display: block;
            inline-size: 100%;
            block-size: 100%;
        }
    `,
    host: { 'aria-hidden': 'true' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrianglesComponent {
    /** How many, relative to the usual. */
    readonly density = input(1);

    private readonly canvas = viewChild.required<ElementRef<HTMLCanvasElement>>('canvas');
    private readonly appearance = inject(AppearanceStore);
    private readonly playState = inject(OsuPlayState);
    private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

    private triangles: Triangle[] = [];
    private colors = { accent: '#f6a', accent2: '#a6f' };
    private width = 0;
    private height = 0;
    private frame = 0;
    private last = 0;
    private visible = true;

    constructor() {
        const destroyRef = inject(DestroyRef);

        afterNextRender(() => {
            const resize = new ResizeObserver(() => this.resize());
            resize.observe(this.host.nativeElement);
            const intersection = new IntersectionObserver(([entry]) => {
                this.visible = entry.isIntersecting;
                this.schedule();
            });
            intersection.observe(this.host.nativeElement);
            const onVisibility = () => this.schedule();
            document.addEventListener('visibilitychange', onVisibility);

            destroyRef.onDestroy(() => {
                resize.disconnect();
                intersection.disconnect();
                document.removeEventListener('visibilitychange', onVisibility);
                cancelAnimationFrame(this.frame);
            });
        });

        // Paused while osu! plays a map, on again after it.
        effect(() => {
            this.playState.inPlay();
            this.schedule();
        });

        // The palette's accents, again when the palette or the theme changes.
        effect(() => {
            this.appearance.palette();
            this.appearance.dark();
            this.appearance.reducedMotion();
            this.appearance.effects();
            this.appearance.quality();
            // After the attributes of <html> are set (another effect).
            requestAnimationFrame(() => {
                this.readColors();
                this.resize();
            });
        });
    }

    /**
     * Drawn once and left still: with reduced motion, or the animated
     * backgrounds off. Paused as they are while osu! plays a map.
     */
    private get still(): boolean {
        return (
            this.playState.inPlay() ||
            !this.appearance.effects() ||
            this.appearance.reducedMotion() ||
            matchMedia('(prefers-reduced-motion: reduce)').matches
        );
    }

    /** The low quality: half as many, at 30 frames a second, at one pixel per CSS pixel. */
    private get low(): boolean {
        return this.appearance.quality() === 'low';
    }

    private readColors() {
        const probe = document.createElement('span');
        this.host.nativeElement.append(probe);
        probe.style.color = 'var(--accent)';
        const accent = getComputedStyle(probe).color;
        probe.style.color = 'var(--accent-2)';
        const accent2 = getComputedStyle(probe).color;
        probe.remove();
        this.colors = { accent, accent2 };
    }

    private resize() {
        const canvas = this.canvas().nativeElement;
        const box = this.host.nativeElement.getBoundingClientRect();
        const ratio = this.low ? 1 : Math.min(devicePixelRatio || 1, 2);
        this.width = box.width;
        this.height = box.height;
        canvas.width = Math.round(box.width * ratio);
        canvas.height = Math.round(box.height * ratio);
        canvas.getContext('2d')?.setTransform(ratio, 0, 0, ratio, 0, 0);

        const count = Math.round(((box.width * box.height) / AREA_PER_TRIANGLE) * this.density() * (this.low ? 0.5 : 1));
        while (this.triangles.length < count) this.triangles.push(this.spawn(true));
        this.triangles.length = count;
        this.draw();
        this.schedule();
    }

    /** A new triangle: anywhere at first, below the box afterwards. */
    private spawn(anywhere: boolean): Triangle {
        const size = Math.max(BASE_SIZE * (1 + 0.35 * gaussian()), 10);
        return {
            x: Math.random(),
            y: anywhere ? Math.random() * 1.2 - 0.2 : 1 + (size * HEIGHT_RATIO) / Math.max(this.height, 1),
            size,
            second: Math.random() < 0.4,
            alpha: 0.08 + Math.random() * 0.22,
        };
    }

    private schedule() {
        cancelAnimationFrame(this.frame);
        if (this.still || !this.visible || document.hidden || !this.width) return;
        this.last = performance.now();
        this.frame = requestAnimationFrame(now => this.tick(now));
    }

    private skipped = false;

    private tick(now: number) {
        // Every other frame only, at the low quality.
        this.skipped = this.low && !this.skipped;
        if (this.skipped) {
            this.frame = requestAnimationFrame(next => this.tick(next));
            return;
        }
        // At most a tenth of a second at once: after a pause they don't jump.
        const seconds = Math.min((now - this.last) / 1000, 0.1);
        this.last = now;
        for (const [i, triangle] of this.triangles.entries()) {
            triangle.y -= (BASE_SPEED * (triangle.size / BASE_SIZE) * seconds) / this.height;
            if (triangle.y < -(triangle.size * HEIGHT_RATIO) / this.height) this.triangles[i] = this.spawn(false);
        }
        this.draw();
        this.frame = requestAnimationFrame(next => this.tick(next));
    }

    private draw() {
        const context = this.canvas().nativeElement.getContext('2d');
        if (!context) return;
        context.clearRect(0, 0, this.width, this.height);
        context.lineWidth = 1.5;
        context.lineJoin = 'round';
        for (const triangle of this.triangles) {
            const x = triangle.x * this.width;
            const y = triangle.y * this.height;
            const h = triangle.size * HEIGHT_RATIO;
            context.globalAlpha = triangle.alpha;
            context.strokeStyle = triangle.second ? this.colors.accent2 : this.colors.accent;
            context.beginPath();
            context.moveTo(x, y);
            context.lineTo(x + triangle.size / 2, y + h);
            context.lineTo(x - triangle.size / 2, y + h);
            context.closePath();
            context.stroke();
        }
        context.globalAlpha = 1;
    }
}
