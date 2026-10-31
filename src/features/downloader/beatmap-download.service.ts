import { inject, Injectable, OnDestroy, signal } from '@angular/core';
import { DownloadQueueStore } from './download-queue.store';
import { UserStore } from './user.store';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

interface DownloadProgress {
  beatmapSetId: number;
  progress: number;
  status: 'downloading' | 'done' | 'failed';
}

@Injectable({
  providedIn: 'root',
})
export class BeatmapDownloadService implements OnDestroy {
  private downloadQueue = inject(DownloadQueueStore);
  private userStore = inject(UserStore);
  private unlisten?: UnlistenFn;

  readonly error = signal<string | null>(null);

  async startDownload() {
    if (this.downloadQueue.isDownloading()) return;

    const pending = this.downloadQueue.queue().filter(i => i.status === 'pending');
    if (pending.length === 0) return;

    if (!this.userStore.user()?.osuSession) return;

    this.error.set(null);
    this.downloadQueue.setDownloading(true);

    this.unlisten = await listen<DownloadProgress>('download:progress', (event) => {
      const { beatmapSetId, progress, status } = event.payload;
      this.downloadQueue.updateStatus(beatmapSetId, status, progress);
    });

    try {
      await invoke('start_downloads', {
        beatmapSetIds: pending.map(i => i.beatmapSetId),
      });
    } catch (e) {
      const msg = typeof e === 'string' ? e : 'Download failed';
      this.error.set(msg);
      for (const item of pending) {
        const q = this.downloadQueue.queue().find(q => q.beatmapSetId === item.beatmapSetId);
        if (q?.status === 'pending') {
          this.downloadQueue.updateStatus(item.beatmapSetId, 'failed');
        }
      }
    } finally {
      this.downloadQueue.setDownloading(false);
      this.unlisten?.();
      this.unlisten = undefined;
    }
  }

  ngOnDestroy() {
    this.unlisten?.();
  }
}
