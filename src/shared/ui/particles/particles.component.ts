import { afterNextRender, ChangeDetectionStrategy, Component, DestroyRef, effect, ElementRef, inject, input, viewChild } from '@angular/core';
import { AppearanceStore } from '../../stores/appearance.store';

interface Particle {
    x: number;
    y: number;
    /** Radius, in CSS pixels. */
    r: number;
    /** Up, in pixels per second. */
    speed: number;
    /** Sway: where in it, and how fast. */
    phase: number;
    sway: number;
    /** Twinkle: where in it, and how fast. */
    twinkle: number;
    rate: number;
    color: 0 | 1 | 2;
}

/** One particle per this many square pixels, at density 1. */
const AREA_PER_PARTICLE = 9000;
const MAX_PARTICLES = 70;

/**
 * Specks of light rising slowly over what is behind, swaying and twinkling,
 * in white and both accents: `<app-particles />` in a positioned element.
 * Light: few of them, paused out of sight, still with reduced motion.
 */
@Component({
    selector: 'app-particles',
    template: '<canvas #canvas></canvas>',
    styles: `
        :host {
            position: absolute;
            inset: 0;
            overflow: hidden;
            pointer-events: none;
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
export class ParticlesComponent {
    readonly density = input(1);

    private readonly canvas = viewChild.required<ElementRef<HTMLCanvasElement>>('canvas');
    private readonly appearance = inject(AppearanceStore);
    private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

    private particles: Particle[] = [];
    private colors = ['#fff', '#f6a', '#a6f'];
    private width = 0;
    private height = 0;
    private frame = 0;
    private last = 0;
    private time = 0;
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

        // The accents of the palette, again when it or the theme changes.
        effect(() => {
            this.appearance.palette();
            this.appearance.dark();
            this.appearance.reducedMotion();
            this.appearance.effects();
            this.appearance.quality();
            requestAnimationFrame(() => {
                this.readColors();
                this.resize();
            });
        });
    }

    /** Drawn once and left still: with reduced motion, or the animated backgrounds off. */
    private get still(): boolean {
        return (
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
        this.colors = ['#fff', accent, accent2];
    }

    private resize() {
        const canvas = this.canvas().nativeElement;
        const box = this.host.nativeElement.getBoundingClientRect();
        // Specks this small don't need every device pixel.
        const ratio = this.low ? 1 : Math.min(devicePixelRatio || 1, 1.5);
        this.width = box.width;
        this.height = box.height;
        canvas.width = Math.round(box.width * ratio);
        canvas.height = Math.round(box.height * ratio);
        canvas.getContext('2d')?.setTransform(ratio, 0, 0, ratio, 0, 0);

        const count = Math.min(Math.round(((box.width * box.height) / AREA_PER_PARTICLE) * this.density() * (this.low ? 0.5 : 1)), MAX_PARTICLES);
        while (this.particles.length < count) this.particles.push(this.spawn(true));
        this.particles.length = count;
        this.draw();
        this.schedule();
    }

    private spawn(anywhere: boolean): Particle {
        const r = 0.6 + Math.random() ** 2 * 1.9;
        const random = Math.random();
        return {
            x: Math.random() * this.width,
            y: anywhere ? Math.random() * this.height : this.height + r * 6,
            r,
            // The bigger, the closer: the faster.
            speed: 6 + r * 9 + Math.random() * 6,
            phase: Math.random() * Math.PI * 2,
            sway: 0.4 + Math.random() * 0.8,
            twinkle: Math.random() * Math.PI * 2,
            rate: 1 + Math.random() * 2,
            color: random < 0.5 ? 0 : random < 0.8 ? 1 : 2,
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
        const seconds = Math.min((now - this.last) / 1000, 0.1);
        this.last = now;
        this.time += seconds;
        for (const [i, p] of this.particles.entries()) {
            p.y -= p.speed * seconds;
            if (p.y < -p.r * 6) this.particles[i] = this.spawn(false);
        }
        this.draw();
        this.frame = requestAnimationFrame(next => this.tick(next));
    }

    private draw() {
        const context = this.canvas().nativeElement.getContext('2d');
        if (!context) return;
        context.clearRect(0, 0, this.width, this.height);
        // Light adds up where specks cross.
        context.globalCompositeOperation = 'lighter';
        for (const p of this.particles) {
            const x = p.x + Math.sin(this.time * p.sway + p.phase) * 6;
            // Fades in from the bottom and out towards the top.
            const height = Math.max(0, Math.min(1, p.y / this.height));
            const twinkle = 0.55 + 0.45 * Math.sin(this.time * p.rate + p.twinkle);
            context.globalAlpha = twinkle * Math.sin(height * Math.PI) * 0.85;
            const glow = context.createRadialGradient(x, p.y, 0, x, p.y, p.r * 4);
            glow.addColorStop(0, this.colors[p.color]);
            glow.addColorStop(0.25, this.colors[p.color]);
            glow.addColorStop(1, 'transparent');
            context.fillStyle = glow;
            context.beginPath();
            context.arc(x, p.y, p.r * 4, 0, Math.PI * 2);
            context.fill();
        }
        context.globalAlpha = 1;
        context.globalCompositeOperation = 'source-over';
    }
}
