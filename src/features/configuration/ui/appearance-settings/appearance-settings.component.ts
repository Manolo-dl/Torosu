import { OptionGroupComponent, OptionGroupItem } from '@shared/ui';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { AppearanceStore, Font, FONTS, PALETTES } from '@shared/stores';

/** How each font is named, in its own letters. */
const FONT_NAMES: Record<Font, string> = { outfit: 'Outfit', nunito: 'Nunito', inter: 'Inter', grotesk: 'Space Grotesk' };
const FONT_FAMILIES: Record<Font, string> = {
    outfit: "'Outfit Variable'",
    nunito: "'Nunito Variable'",
    inter: "'Inter Variable'",
    grotesk: "'Space Grotesk Variable'",
};

/** The palette, the font and the theme: they change the app as they are picked. */
@Component({
    selector: 'app-appearance-settings',
    imports: [OptionGroupComponent, TranslocoPipe],
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

    readonly themes: OptionGroupItem[] = [
        { label: 'settings.appearance.themes.dark', icon: 'moon', translate: true },
        { label: 'settings.appearance.themes.light', icon: 'sun', translate: true },
    ];
}
