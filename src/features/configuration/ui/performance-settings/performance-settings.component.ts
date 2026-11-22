import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { AppearanceStore, EFFECTS_QUALITIES } from '@shared/stores';
import { OptionGroupComponent, OptionGroupItem, SectionTitleComponent, SwitchComponent } from '@shared/ui';
import { TuiCell, TuiTitle } from '@taiga-ui/core';

/**
 * How much moves and what it costs: the effects, their quality, the
 * transitions and the music of osu!. Applied as they are changed; kept with
 * the look of the window, not in the tools' settings.
 */
@Component({
    selector: 'app-performance-settings',
    imports: [OptionGroupComponent, SectionTitleComponent, SwitchComponent, TranslocoPipe, TuiCell, TuiTitle],
    templateUrl: './performance-settings.component.html',
    styleUrl: './performance-settings.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PerformanceSettingsComponent {
    readonly appearance = inject(AppearanceStore);

    readonly qualities: OptionGroupItem[] = EFFECTS_QUALITIES.map(quality => ({
        label: `settings.performance.qualities.${quality}`,
        translate: true,
    }));
    readonly qualityIndex = computed(() => EFFECTS_QUALITIES.indexOf(this.appearance.quality()));

    setQuality(index: number) {
        this.appearance.setQuality(EFFECTS_QUALITIES[index]);
    }

    checked(event: Event): boolean {
        return (event.target as HTMLInputElement).checked;
    }
}
