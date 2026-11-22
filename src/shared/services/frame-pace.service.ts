import { computed, inject, Injectable, signal } from '@angular/core';
import { AppearanceStore } from '../stores/appearance.store';

/** Frames a second at the low quality, and without the focus (with it or not). */
const LOW_QUALITY_FPS = 30;
const AWAY_FPS = 15;

/**
 * One pace for what is drawn from script just to look nice: the music's
 * bars and pulse, the canvases (triangles, specks of light). WebKit costs
 * the same for every frame whatever moves in it, so they skip the same
 * frames: each asks `isFrame(now)` with its requestAnimationFrame time, the
 * same for every callback of a frame.
 *
 * Every frame with the focus and the high quality; 30 a second at the low
 * quality; 15 without the focus (osu! active, mostly: Wayland doesn't tell a
 * window it is out of sight, but it does tell it lost the focus).
 *
 * Not the CSS ornaments (the orbit, the glow...): the compositor moves them
 * on its own thread, and moving them from script instead made WebKit style
 * and paint them on the main thread, which cost more than it saved.
 */
@Injectable({ providedIn: 'root' })
export class FramePace {
    private readonly appearance = inject(AppearanceStore);
    private readonly focused = signal(document.hasFocus());

    /** The least time between two frames drawn, in ms; 0 for every frame. */
    readonly frameMs = computed(() => {
        if (!this.focused()) return 1000 / AWAY_FPS;
        return this.appearance.quality() === 'low' ? 1000 / LOW_QUALITY_FPS : 0;
    });

    /** The last frame asked about, and whether it was one to draw. */
    private frameAt = -1;
    private drawFrame = true;
    /** The slot of the last frame drawn: one is drawn whenever the slot changes. */
    private slot = -1;

    constructor() {
        addEventListener('focus', () => this.focused.set(true));
        addEventListener('blur', () => this.focused.set(false));
    }

    /** Whether to draw at `now`, a requestAnimationFrame time: the same answer for all asking that frame. */
    isFrame(now: number): boolean {
        if (now === this.frameAt) return this.drawFrame;
        this.frameAt = now;
        const ms = this.frameMs();
        if (!ms) {
            this.drawFrame = true;
            return true;
        }
        // A little early is on time: frames don't come exactly every 1/60 s.
        const slot = Math.floor((now + 4) / ms);
        this.drawFrame = slot !== this.slot;
        if (this.drawFrame) this.slot = slot;
        return this.drawFrame;
    }
}
