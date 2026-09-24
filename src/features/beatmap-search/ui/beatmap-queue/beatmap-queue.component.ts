import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { DecimalPipe } from '@angular/common';
import { BeatmapStore } from '@entities/beatmap';
import { BeatmapDownloadService } from '@features/beatmap-search';

@Component({
  selector: 'app-beatmap-queue',
  imports: [DecimalPipe],
  templateUrl: './beatmap-queue.component.html',
  styleUrl: './beatmap-queue.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapQueueComponent {
  readonly beatmapStore = inject(BeatmapStore);
  readonly downloader = inject(BeatmapDownloadService);

  readonly pending = computed(() => this.beatmapStore.queue().filter(i => i.status === 'pending').length);
  readonly done    = computed(() => this.beatmapStore.queue().filter(i => i.status === 'done').length);
  readonly failed  = computed(() => this.beatmapStore.queue().filter(i => i.status === 'failed').length);

  async startDownload() {
    await this.downloader.startDownload();
  }
}
