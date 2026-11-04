import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { BeatmapExporterStore } from '../../stores/beatmap-exporter.store';
import { BeatmapSetListComponent } from '@entities/osu-db/ui';
import { TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-beatmap-preview',
  imports: [BeatmapSetListComponent, TuiCardLarge, TuiTitle, TranslocoPipe],
  template: `
    <div appearance="floating" tuiCardLarge="compact">
      <h3 tuiTitle>{{ 'exportSongs.preview' | transloco }}</h3>
      <app-beatmap-set-list
        [sets]="store.filteredBeatmaps()"
        emptyText="exportSongs.empty"
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
