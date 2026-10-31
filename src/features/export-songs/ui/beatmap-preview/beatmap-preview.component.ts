import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { BeatmapExporterStore } from '../../stores/beatmap-exporter.store';
import { BeatmapSetListComponent } from '@entities/osu-db/ui';
import { TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-beatmap-preview',
  imports: [BeatmapSetListComponent, TuiCardLarge, TuiTitle],
  template: `
    <div appearance="floating" tuiCardLarge="compact">
      <h3 tuiTitle>Preview</h3>
      <app-beatmap-set-list
        [sets]="store.filteredBeatmaps()"
        emptyText="No beatmaps match the current filters"
        listHeight="calc(100vh - 26rem)"
      />
    </div>
  `,
  styleUrl: './beatmap-preview.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapPreviewComponent {
  readonly store = inject(BeatmapExporterStore);
}
