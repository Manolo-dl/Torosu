import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { AppearanceStore, FONTS, PALETTES } from '@shared/stores';
import { TuiCell, TuiTitle } from '@taiga-ui/core';
import { TuiSwitch } from '@taiga-ui/kit';

/** How each font is named, in its own letters. */
const FONT_NAMES: Record<(typeof FONTS)[number], string> = { nunito: 'Nunito', outfit: 'Outfit', inter: 'Inter' };
const FONT_FAMILIES: Record<(typeof FONTS)[number], string> = {
    nunito: "'Nunito Variable'",
    outfit: "'Outfit Variable'",
    inter: "'Inter Variable'",
};

/** The palette, the font and the motion: they change the app as they are picked. */
@Component({
    selector: 'app-appearance-settings',
    imports: [TranslocoPipe, TuiCell, TuiSwitch, TuiTitle],
    templateUrl: './appearance-settings.component.html',
    styleUrl: './appearance-settings.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AppearanceSettingsComponent {
    readonly appearance = inject(AppearanceStore);
    readonly palettes = PALETTES;
    readonly fonts = FONTS;
    readonly fontNames = FONT_NAMES;
    readonly fontFamilies = FONT_FAMILIES;

    setReducedMotion(event: Event) {
        this.appearance.setReducedMotion((event.target as HTMLInputElement).checked);
    }
}
