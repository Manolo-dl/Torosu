import { ButtonComponent, BadgeComponent, CardComponent, IconComponent, NoticeComponent, ProgressComponent } from '@shared/ui';
import { beatmapsetUrl } from '@shared/utils';
import { openUrl } from '@tauri-apps/plugin-opener';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { DecimalPipe } from '@angular/common';
import { BeatmapDownloadService } from '../../beatmap-download.service';
import { DownloadStatus } from '../../download-queue.model';
import { DownloadQueueStore } from '../../download-queue.store';
import { TuiScrollbar } from '@taiga-ui/core';

@Component({
  selector: 'app-beatmap-queue',
  imports: [IconComponent, ProgressComponent, DecimalPipe, TuiScrollbar, TranslocoPipe, ButtonComponent, BadgeComponent, CardComponent, NoticeComponent],
  templateUrl: './beatmap-queue.component.html',
  styleUrl: './beatmap-queue.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapQueueComponent {
  readonly downloadQueue = inject(DownloadQueueStore);
  readonly downloader = inject(BeatmapDownloadService);

  readonly stateIcons: Record<DownloadStatus, string> = {
    pending: 'clock',
    downloading: 'loader-circle',
    done: 'circle-check',
    failed: 'circle-x',
  };

  readonly pending = computed(() => this.downloadQueue.queue().filter(i => i.status === 'pending').length);
  readonly done    = computed(() => this.downloadQueue.queue().filter(i => i.status === 'done').length);
  readonly failed  = computed(() => this.downloadQueue.queue().filter(i => i.status === 'failed').length);

  /** How far the whole queue has got, 0 to 100: the finished, and the share done of those under way. */
  readonly overall = computed(() => {
    const queue = this.downloadQueue.queue();
    if (queue.length === 0) return 0;
    const finished = queue.reduce(
      (sum, item) => sum + (item.status === 'done' || item.status === 'failed' ? 1 : item.status === 'downloading' ? item.progress / 100 : 0),
      0,
    );
    return (finished / queue.length) * 100;
  });

  openPage(beatmapSetId: number) {
    openUrl(beatmapsetUrl(beatmapSetId)).catch(e => console.error('failed to open the beatmap set page', e));
  }

  async startDownload() {
    await this.downloader.startDownload();
  }
}
