import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component } from '@angular/core';
import { PageHeaderComponent } from '@shared/ui';
import { BeatmapFiltersComponent, BeatmapSummaryComponent, BeatmapPreviewComponent, BeatmapExportControlsComponent } from '@features/export-songs'; 

@Component({
  selector: 'app-export-song-folder',
  imports: [BeatmapFiltersComponent, BeatmapSummaryComponent, BeatmapPreviewComponent, BeatmapExportControlsComponent, PageHeaderComponent, TranslocoPipe],
  templateUrl: './export-song-folder-page.component.html',
  styleUrl: './export-song-folder-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExportSongFolderPageComponent {}
