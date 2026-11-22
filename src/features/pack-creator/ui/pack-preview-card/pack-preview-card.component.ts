import { ButtonComponent, IconComponent, StarBadgeComponent, TooltipDirective } from '@shared/ui';
import { BeatmapCoverService, OsuDbStore } from '@entities/osu-db';
import { modeColor, modeIcon } from '@shared/utils';
import { AppearanceStore } from '@shared/stores';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, computed, effect, inject, input, signal } from '@angular/core';
import { form, required, FormField } from '@angular/forms/signals';
import { SelectedDiff } from '../../pack.model';
import { PackStore } from '../../pack.store';
import { TuiInput, tuiTextfieldOptionsProvider } from '@taiga-ui/core';

@Component({
  selector: 'app-pack-preview-card',
  imports: [FormField, TuiInput, TranslocoPipe, ButtonComponent, IconComponent, StarBadgeComponent, TooltipDirective],
  providers: [tuiTextfieldOptionsProvider({ size: signal('s'), cleaner: signal(false) })],
  templateUrl: './pack-preview-card.component.html',
  styleUrl: './pack-preview-card.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PackPreviewCardComponent {

  readonly diff = input.required<SelectedDiff>();
  readonly store = inject(PackStore);
  private readonly osuDb = inject(OsuDbStore);

  /** The diff it comes from, and its set, in osu!.db. */
  readonly original = computed(() => this.osuDb.diffsByMd5().get(this.diff().md5) ?? null);
  readonly set = computed(() => this.osuDb.beatmapSetsByMd5().get(this.diff().md5) ?? null);

  /** Its cover, from its own background (`BeatmapCoverService`); its mode's icon without one. */
  readonly cover = signal<string | null>(null);
  private readonly covers = inject(BeatmapCoverService);
  private readonly appearance = inject(AppearanceStore);
  readonly modeIcon = computed(() => modeIcon(this.original()?.mode ?? 3));
  readonly modeColor = computed(() => modeColor(this.original()?.mode ?? 3));

  private readonly model = signal({ newDiffName: '' });
  readonly nameForm = form(
    this.model,
    (schemaPath) => {
      required(schemaPath.newDiffName, { message: 'packs.diffNameRequired' });
    }
  );

  // Shown as an icon with a tooltip instead of a line of text so every row keeps the
  // same height, which the virtual scroll in the pack preview relies on.
  // A translation key.
  readonly error = computed(() => {
    if (this.store.duplicateNames().has(this.diff().md5)) {
      return 'packs.diffNameTaken';
    }
    return this.nameForm.newDiffName().errors()[0]?.message ?? null;
  });

  constructor() {
    effect(onCleanup => {
      const { folderName, fileName } = this.diff();
      this.appearance.covers();
      this.cover.set(null);
      let current = true;
      onCleanup(() => (current = false));
      this.covers.cover(folderName, fileName).then(url => current && this.cover.set(url));
    });

    effect(() => {
      this.model.set({ newDiffName: this.diff().newDiffName });
    });

    effect(() => {
      this.store.updateDiffName(this.diff(), this.nameForm.newDiffName().value());
    });
  }

  onRemove() {
    this.store.removeDiff(this.diff().md5);
  }
}
