import { effect, Injectable, signal } from '@angular/core';

/**
 * Whether osu! is playing a map (or a replay), as the osu! reader tells it.
 * No one looks at Torosu then and osu! wants the CPU, so what loops just to
 * look nice stands still meanwhile: the canvases (triangles, specks of light)
 * through `inPlay`, the CSS ornaments through data-playing on <html>.
 */
@Injectable({ providedIn: 'root' })
export class OsuPlayState {
    readonly inPlay = signal(false);

    constructor() {
        effect(() => {
            document.documentElement.dataset['playing'] = this.inPlay() ? 'on' : 'off';
        });
    }
}
