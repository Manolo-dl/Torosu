import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { DownloadQueueStore } from '../../download-queue.store';
import { TxtParserService } from '../../txt-parser.service';
import { TxtDropzoneComponent } from '@shared/ui';

@Component({
  selector: 'app-beatmap-import',
  imports: [TxtDropzoneComponent],
  templateUrl: './beatmap-import.component.html',
  styleUrl: './beatmap-import.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapImportComponent {
  private downloadQueue = inject(DownloadQueueStore);
  private parser = inject(TxtParserService);

  addToQueue(content: string) {
    this.downloadQueue.addToQueue(this.parser.parse(content));
  }
}
