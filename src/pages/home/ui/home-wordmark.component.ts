import { ChangeDetectionStrategy, Component } from '@angular/core';

/**
 * "Torosu", its letters falling like notes onto a judgement line, and
 * hopping as keys under the pointer. The text is read whole (aria-label).
 */
@Component({
    selector: 'app-home-wordmark',
    template: `
        @for (letter of letters; track $index) {
            <span class="letter" [style.--i]="$index"><span class="key">{{ letter }}</span></span>
        }
    `,
    styleUrl: './home-wordmark.component.less',
    host: { role: 'img', 'aria-label': 'Torosu' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HomeWordmarkComponent {
    readonly letters = [...'Torosu'];
}
