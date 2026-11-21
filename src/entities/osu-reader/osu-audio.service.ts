import { effect, inject, Injectable, untracked } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { LanguageService } from '@shared/i18n';
import { OsuReaderStore } from './osu-reader.store';

/** `AudioFrame` in osu_reader/audio.rs. */
interface AudioFrame {
    playTime: number;
    spectrum?: number[];
}

/** The decoded song is kept at this rate, in one channel: enough up to 11 kHz. */
const SAMPLE_RATE = 22050;
const FFT_SIZE = 1024;
/** The bars go from here to there, on a log scale. */
const LOW_HZ = 45;
const HIGH_HZ = 10000;
/** A selected map's song is read once the selection has rested this long. */
const LOAD_DELAY_MS = 350;
/** osu!'s own spectrum (main menu) is trusted for this long after it came. */
const SPECTRUM_FRESH_MS = 250;

/**
 * The music osu! is playing, as levels of `count` bands low to high, for
 * the visualizer in the title bar. osu! says where its song is (often, by
 * `osu-audio`); between readings that is extrapolated at the pace it was
 * going. On the main menu osu! gives its own spectrum; elsewhere the song of
 * the selected map is decoded here and its spectrum taken at that point.
 */
@Injectable({ providedIn: 'root' })
export class OsuAudioService {
    private readonly reader = inject(OsuReaderStore);
    private readonly language = inject(LanguageService);

    private unlisten: UnlistenFn | null = null;
    private enabled = false;

    /** The last reading, when it came, and the pace of the song (1 playing, 0 paused). */
    private playTime = 0;
    private readAt = 0;
    private rate = 0;
    private spectrum: number[] | null = null;
    private spectrumAt = 0;

    /** The selected map's song, decoded, and its path. */
    private samples: Float32Array | null = null;
    private audioPath: string | null = null;
    private loading = 0;

    private readonly fft = new Fft(FFT_SIZE);

    constructor() {
        // The song of the map selected, once the selection rests.
        let timer: ReturnType<typeof setTimeout> | undefined;
        effect(() => {
            const path = this.reader.data()?.paths?.audio ?? null;
            untracked(() => {
                clearTimeout(timer);
                if (path === this.audioPath) return;
                timer = setTimeout(() => this.loadSong(path), LOAD_DELAY_MS);
            });
        });
    }

    /** Starts (or stops) following osu!'s music; the reader starts with it. */
    async setEnabled(enabled: boolean) {
        if (enabled === this.enabled) return;
        this.enabled = enabled;
        if (enabled) {
            // The reader says when osu! is found: in the language in use, once it is.
            await this.language.ready;
            if (!this.enabled) return;
            this.reader.connect();
            this.unlisten ??= await listen<AudioFrame>('osu-audio', event => this.onFrame(event.payload));
        }
        invoke('watch_osu_audio', { enabled }).catch(error => console.warn('watch_osu_audio failed', error));
        // The song is decoded only while it is wanted; dropped when not.
        if (enabled) this.loadSong(this.reader.data()?.paths?.audio ?? null);
        else {
            this.loading++;
            this.samples = null;
            this.audioPath = null;
        }
    }

    /** Whether something is playing: the song moves, or osu! sent its spectrum lately. */
    playing(now = performance.now()): boolean {
        return this.reader.connected() && (this.rate > 0.05 || now - this.spectrumAt < SPECTRUM_FRESH_MS);
    }

    /** The levels of `count` bands, 0 to 1, low to high, now; all 0 when nothing plays. */
    levels(count: number, now = performance.now()): Float32Array {
        const levels = new Float32Array(count);
        if (!this.reader.connected()) return levels;

        if (this.spectrum && now - this.spectrumAt < SPECTRUM_FRESH_MS) {
            const bands = this.spectrum;
            for (let i = 0; i < count; i++) {
                const from = Math.floor((i * bands.length) / count);
                const to = Math.max(from + 1, Math.floor(((i + 1) * bands.length) / count));
                let peak = 0;
                for (let b = from; b < to; b++) peak = Math.max(peak, bands[b]);
                levels[i] = Math.min(1, Math.sqrt(Math.max(0, peak)) * 1.4);
            }
            return levels;
        }

        if (!this.samples || this.rate <= 0.05) return levels;
        const time = this.playTime + this.rate * (now - this.readAt);
        const start = Math.round((time / 1000) * SAMPLE_RATE) - FFT_SIZE / 2;
        if (start < 0 || start + FFT_SIZE > this.samples.length) return levels;
        const magnitudes = this.fft.magnitudes(this.samples, start);

        const binHz = SAMPLE_RATE / FFT_SIZE;
        for (let i = 0; i < count; i++) {
            const low = LOW_HZ * Math.pow(HIGH_HZ / LOW_HZ, i / count);
            const high = LOW_HZ * Math.pow(HIGH_HZ / LOW_HZ, (i + 1) / count);
            const from = Math.max(1, Math.floor(low / binHz));
            const to = Math.max(from + 1, Math.ceil(high / binHz));
            let peak = 0;
            for (let b = from; b < to && b < magnitudes.length; b++) peak = Math.max(peak, magnitudes[b]);
            // To decibels, -62 dB the floor; the highs a little lifted, as they are quieter.
            const db = 20 * Math.log10(peak + 1e-9) + (i / count) * 9;
            levels[i] = Math.min(1, Math.max(0, (db + 62) / 52));
        }
        return levels;
    }

    private onFrame(frame: AudioFrame) {
        const now = performance.now();
        if (frame.spectrum) {
            this.spectrum = frame.spectrum;
            this.spectrumAt = now;
        }
        const elapsed = now - this.readAt;
        const moved = frame.playTime - this.playTime;
        // The pace: how far the song went since the last reading (DT plays it
        // faster); a jump (another song, a seek) starts again at normal speed.
        let pace = elapsed > 0 ? moved / elapsed : 0;
        if (pace < 0 || pace > 2) pace = moved === 0 ? 0 : 1;
        this.rate = moved === 0 ? 0 : this.rate === 0 ? pace : this.rate * 0.7 + pace * 0.3;
        this.playTime = frame.playTime;
        this.readAt = now;
    }

    /** Reads and decodes the song `path`; a newer request wins. */
    private async loadSong(path: string | null) {
        this.audioPath = path;
        this.samples = null;
        if (!path || !this.enabled) return;
        const request = ++this.loading;
        try {
            const bytes = await invoke<ArrayBuffer>('read_songs_file', { path });
            if (request !== this.loading) return;
            const context = new OfflineAudioContext(1, 1, SAMPLE_RATE);
            const buffer = await context.decodeAudioData(bytes);
            if (request !== this.loading) return;
            this.samples = buffer.getChannelData(0);
        } catch (error) {
            console.warn('could not decode the song for the visualizer', error);
        }
    }
}

/** A radix-2 FFT of a fixed size, with a Hann window, reusing its buffers. */
class Fft {
    private readonly window: Float32Array;
    private readonly reversed: Uint32Array;
    private readonly cos: Float32Array;
    private readonly sin: Float32Array;
    private readonly re: Float32Array;
    private readonly im: Float32Array;
    private readonly out: Float32Array;

    constructor(private readonly size: number) {
        this.window = Float32Array.from({ length: size }, (_, i) => 0.5 - 0.5 * Math.cos((2 * Math.PI * i) / (size - 1)));
        const bits = Math.log2(size);
        this.reversed = Uint32Array.from({ length: size }, (_, i) => {
            let r = 0;
            for (let b = 0; b < bits; b++) r = (r << 1) | ((i >> b) & 1);
            return r;
        });
        this.cos = Float32Array.from({ length: size / 2 }, (_, i) => Math.cos((-2 * Math.PI * i) / size));
        this.sin = Float32Array.from({ length: size / 2 }, (_, i) => Math.sin((-2 * Math.PI * i) / size));
        this.re = new Float32Array(size);
        this.im = new Float32Array(size);
        this.out = new Float32Array(size / 2);
    }

    /** The amplitude (0 to about 1) of each frequency of `samples` from `start`. */
    magnitudes(samples: Float32Array, start: number): Float32Array {
        const { size, re, im } = this;
        for (let i = 0; i < size; i++) {
            re[this.reversed[i]] = samples[start + i] * this.window[i];
            im[this.reversed[i]] = 0;
        }
        for (let half = 1; half < size; half *= 2) {
            const step = size / (half * 2);
            for (let i = 0; i < size; i += half * 2) {
                for (let j = 0; j < half; j++) {
                    const c = this.cos[j * step];
                    const s = this.sin[j * step];
                    const a = i + j;
                    const b = a + half;
                    const tr = re[b] * c - im[b] * s;
                    const ti = re[b] * s + im[b] * c;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
        }
        // A Hann window halves the amplitude: scaled back so a full sine is about 1.
        const scale = 4 / size;
        for (let i = 0; i < size / 2; i++) this.out[i] = Math.hypot(re[i], im[i]) * scale;
        return this.out;
    }
}
