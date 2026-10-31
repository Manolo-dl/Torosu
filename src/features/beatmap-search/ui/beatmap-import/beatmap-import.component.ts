import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { BeatmapStore } from '@entities/beatmap';
import { TxtParserService } from '@features/beatmap-search/txt-parser.service';
import { TxtDropzoneComponent } from '@shared/ui';

@Component({
  selector: 'app-beatmap-import',
  imports: [TxtDropzoneComponent],
  templateUrl: './beatmap-import.component.html',
  styleUrl: './beatmap-import.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapImportComponent {
  private beatmapStore = inject(BeatmapStore);
  private parser = inject(TxtParserService);

  addToQueue(content: string) {
    this.beatmapStore.addToQueue(this.parser.parse(content));
  }
}
