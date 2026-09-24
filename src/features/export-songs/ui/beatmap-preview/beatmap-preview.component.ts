import { AsyncPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { toObservable } from '@angular/core/rxjs-interop';
import { BeatmapExporterStore } from '@features/export-songs/stores/beatmap-exporter.store';
import { BeatmapSetListComponent } from '@shared/ui';

@Component({
  selector: 'app-beatmap-preview',
  imports: [AsyncPipe, BeatmapSetListComponent],
  template: `
    <div class="songs-panel">
      <div class="panel-header">
        <span class="section-title">Preview</span>
      </div>
      <app-beatmap-set-list
        [sets]="(filteredBeatmaps$ | async) ?? []"
        emptyText="No beatmaps match the current filters"
      />
    </div>
  `,
  styleUrl: './beatmap-preview.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapPreviewComponent {
  readonly store = inject(BeatmapExporterStore);
  readonly filteredBeatmaps$ = toObservable(this.store.filteredBeatmaps);
}
