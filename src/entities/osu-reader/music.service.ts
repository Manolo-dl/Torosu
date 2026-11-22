import { inject, Injectable } from '@angular/core';
import { OsuAudioService } from './osu-audio.service';

/** What the music is doing, this frame. */
export interface Music {
    /** Each band, low to high, 0 to 1, smoothed: what the bars show. */
    levels: Float32Array;
    /** How loud overall, 0 to 1, smoothed. */
    energy: number;
    /** A bass hit: jumps up on one and falls off quickly, 0 to 1. */
    beat: number;
}

/** A listener returns whether it is still moving (falling back to rest). */
export type MusicListener = (music: Music, now: number) => boolean;

export const MUSIC_BANDS = 40;
/** How fast the bands rise to a level and fall from it, per second. */
const RISE = 30;
const FALL = 7;
/** How fast a beat fades, per second. */
const BEAT_FALL = 5;
/** While nothing plays and everything rests, how often to look again. */
const IDLE_MS = 300;
/**
 * Frames a second at the low quality, or while another window is active
 * (osu!, mostly): every frame costs WebKit the same whatever moves in it,
 * about a fifth of a core at 60 a second between the page and the window.
 * Both: fewer still.
 */
const REDUCED_FPS = 30;
const LEAST_FPS = 20;

/**
 * One loop for everything that moves with osu!'s music (the bars, the logo,
 * the glow behind the pages): the spectrum is taken once a frame and shared.
 * It runs every frame only while there is music or something still falls
 * back to rest, and only while someone listens.
 */
@Injectable({ providedIn: 'root' })
export class MusicService {
    private readonly audio = inject(OsuAudioService);
    private readonly listeners = new Set<MusicListener>();
    private readonly music: Music = { levels: new Float32Array(MUSIC_BANDS), energy: 0, beat: 0 };
    /** The bass, averaged slowly: a beat is the bass going well over it. */
    private bassAverage = 0;
    private frame = 0;
    private idle: ReturnType<typeof setTimeout> | undefined;
    private last = 0;
    /** When the last frame was drawn, to keep to the rate. */
    private drawn = 0;
    /** The low quality setting: fewer frames. */
    lowQuality = false;

    constructor() {
        document.addEventListener('visibilitychange', () => this.wake());
    }

    /** The least time between two frames drawn, in ms; 0 for every frame. */
    private get frameMs(): number {
        // Wayland doesn't tell a window it is out of sight, but it does tell it lost the focus.
        const reduced = this.lowQuality || !document.hasFocus();
        if (!reduced) return 0;
        return 1000 / (this.lowQuality && !document.hasFocus() ? LEAST_FPS : REDUCED_FPS);
    }

    /** Called every frame with the music; returns a function that stops it. */
    listen(listener: MusicListener): () => void {
        this.listeners.add(listener);
        this.wake();
        return () => {
            this.listeners.delete(listener);
            if (this.listeners.size === 0) this.sleep();
        };
    }

    /** Looks again now (osu! opened, a setting changed). */
    wake() {
        this.sleep();
        if (document.hidden || this.listeners.size === 0) return;
        this.last = performance.now();
        this.frame = requestAnimationFrame(now => this.tick(now));
    }

    private sleep() {
        cancelAnimationFrame(this.frame);
        clearTimeout(this.idle);
    }

    private tick(now: number) {
        // Fewer frames at the low quality or without the focus: skipped,
        // nothing changes on the page and WebKit draws nothing. A little
        // early is on time: frames don't come exactly every 1/60 s.
        if (now - this.drawn < this.frameMs - 4) {
            this.frame = requestAnimationFrame(next => this.tick(next));
            return;
        }
        this.drawn = now;

        const seconds = Math.min((now - this.last) / 1000, 0.1);
        this.last = now;
        const { levels } = this.music;
        const raw = this.audio.levels(MUSIC_BANDS, now);

        let sum = 0;
        let bass = 0;
        const bassBands = Math.round(MUSIC_BANDS * 0.18);
        for (let i = 0; i < MUSIC_BANDS; i++) {
            const speed = raw[i] > levels[i] ? RISE : FALL;
            levels[i] += (raw[i] - levels[i]) * Math.min(1, speed * seconds);
            sum += levels[i];
            if (i < bassBands) bass += raw[i];
        }
        bass /= bassBands;
        this.music.energy = sum / MUSIC_BANDS;

        // A hit: the bass clearly above its recent average.
        this.bassAverage += (bass - this.bassAverage) * Math.min(1, 2.5 * seconds);
        const hit = Math.max(0, (bass - this.bassAverage - 0.06) * 3.2);
        this.music.beat = Math.min(1, Math.max(hit, this.music.beat - BEAT_FALL * seconds * this.music.beat - 0.4 * seconds));

        let moving = this.music.energy > 0.005 || this.music.beat > 0.005;
        for (const listener of this.listeners) moving = listener(this.music, now) || moving;

        if (moving || this.audio.playing(now)) {
            this.frame = requestAnimationFrame(next => this.tick(next));
        } else {
            this.idle = setTimeout(() => this.wake(), IDLE_MS);
        }
    }
}
