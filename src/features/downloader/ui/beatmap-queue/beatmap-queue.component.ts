import { ButtonComponent, BadgeComponent, CardComponent, NoticeComponent, BadgeTone } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { DecimalPipe } from '@angular/common';
import { BeatmapDownloadService } from '../../beatmap-download.service';
import { DownloadStatus } from '../../download-queue.model';
import { DownloadQueueStore } from '../../download-queue.store';
import { TuiScrollbar } from '@taiga-ui/core';
import { TuiProgress } from '@taiga-ui/kit';

@Component({
  selector: 'app-beatmap-queue',
  imports: [DecimalPipe, TuiScrollbar, TuiProgress, TranslocoPipe, ButtonComponent, BadgeComponent, CardComponent, NoticeComponent],
  templateUrl: './beatmap-queue.component.html',
  styleUrl: './beatmap-queue.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapQueueComponent {
  readonly downloadQueue = inject(DownloadQueueStore);
  readonly downloader = inject(BeatmapDownloadService);

  readonly statusTone: Record<DownloadStatus, BadgeTone> = {
    pending: 'neutral',
    downloading: 'info',
    done: 'success',
    failed: 'danger',
  };

  readonly pending = computed(() => this.downloadQueue.queue().filter(i => i.status === 'pending').length);
  readonly done    = computed(() => this.downloadQueue.queue().filter(i => i.status === 'done').length);
  readonly failed  = computed(() => this.downloadQueue.queue().filter(i => i.status === 'failed').length);

  async startDownload() {
    await this.downloader.startDownload();
  }
}
