import { Directive, effect, ElementRef, inject, input } from '@angular/core';
import { AppearanceStore } from '@shared/stores';
import { MusicService } from '../../music.service';

/**
 * An element moving with osu!'s music: `beat` swells it on each bass hit
 * (the logo, as osu!'s cookie does), `glow` brings it in as the music gets
 * loud (a light behind the pages). Only its transform and opacity change,
 * set on the element itself: nothing else is styled again. Off with the
 * setting, or reduced motion; `glow` is hidden then.
 */
@Directive({ selector: '[appMusicPulse]' })
export class MusicPulseDirective {
    /** `beat` with the attribute alone. */
    readonly appMusicPulse = input<'beat' | 'glow', 'beat' | 'glow' | ''>('beat', { transform: mode => mode || 'beat' });
    /** How much, 1 being the usual. */
    readonly pulseStrength = input(1);

    private readonly element = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    private readonly music = inject(MusicService);
    private readonly appearance = inject(AppearanceStore);

    constructor() {
        effect(onCleanup => {
            const mode = this.appMusicPulse();
            const strength = this.pulseStrength();
            const active = this.appearance.musicReactive() && !this.appearance.reducedMotion();
            this.element.style.willChange = active ? 'transform, opacity' : '';
            this.rest(mode);
            if (!active) return;

            const stop = this.music.listen(({ beat, energy }) => {
                if (mode === 'beat') {
                    const scale = 1 + beat * 0.14 * strength;
                    this.element.style.transform = `scale(${scale.toFixed(4)})`;
                    return beat > 0.004;
                }
                const opacity = Math.min(1, (energy * 1.5 + beat * 0.45) * strength);
                this.element.style.opacity = opacity.toFixed(3);
                this.element.style.transform = `scale(${(1 + beat * 0.035 * strength).toFixed(4)})`;
                return opacity > 0.004;
            });
            onCleanup(() => {
                stop();
                this.rest(mode);
            });
        });
    }

    private rest(mode: 'beat' | 'glow') {
        this.element.style.transform = '';
        this.element.style.opacity = mode === 'glow' ? '0' : '';
    }
}
