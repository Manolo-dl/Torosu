import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { DecimalPipe } from '@angular/common';
import { BeatmapDownloadService } from '../../beatmap-download.service';
import { DownloadStatus } from '../../download-queue.model';
import { DownloadQueueStore } from '../../download-queue.store';
import { TuiButton, TuiNotificationDirective, TuiScrollbar } from '@taiga-ui/core';
import { TuiBadge, TuiButtonLoading, TuiProgress } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-beatmap-queue',
  imports: [DecimalPipe, TuiButton, TuiButtonLoading, TuiNotificationDirective, TuiScrollbar, TuiBadge, TuiProgress, TuiCardLarge],
  templateUrl: './beatmap-queue.component.html',
  styleUrl: './beatmap-queue.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapQueueComponent {
  readonly downloadQueue = inject(DownloadQueueStore);
  readonly downloader = inject(BeatmapDownloadService);

  readonly statusAppearance: Record<DownloadStatus, string> = {
    pending: 'neutral',
    downloading: 'info',
    done: 'positive',
    failed: 'negative',
  };

  readonly pending = computed(() => this.downloadQueue.queue().filter(i => i.status === 'pending').length);
  readonly done    = computed(() => this.downloadQueue.queue().filter(i => i.status === 'done').length);
  readonly failed  = computed(() => this.downloadQueue.queue().filter(i => i.status === 'failed').length);

  async startDownload() {
    await this.downloader.startDownload();
  }
}
