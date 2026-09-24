import { inject, Injectable, OnDestroy, signal } from '@angular/core';
import { BeatmapStore } from '@entities/beatmap';
import { UserStore } from '@entities/user';
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
  private beatmapStore = inject(BeatmapStore);
  private userStore = inject(UserStore);
  private unlisten?: UnlistenFn;

  readonly error = signal<string | null>(null);

  async startDownload() {
    if (this.beatmapStore.isDownloading()) return;

    const pending = this.beatmapStore.queue().filter(i => i.status === 'pending');
    if (pending.length === 0) return;

    if (!this.userStore.user()?.osuSession) return;

    this.error.set(null);
    this.beatmapStore.setDownloading(true);

    this.unlisten = await listen<DownloadProgress>('download:progress', (event) => {
      const { beatmapSetId, progress, status } = event.payload;
      this.beatmapStore.updateStatus(beatmapSetId, status, progress);
    });

    try {
      await invoke('start_downloads', {
        beatmapSetIds: pending.map(i => i.beatmapSetId),
      });
    } catch (e) {
      const msg = typeof e === 'string' ? e : 'Download failed';
      this.error.set(msg);
      for (const item of pending) {
        const q = this.beatmapStore.queue().find(q => q.beatmapSetId === item.beatmapSetId);
        if (q?.status === 'pending') {
          this.beatmapStore.updateStatus(item.beatmapSetId, 'failed');
        }
      }
    } finally {
      this.beatmapStore.setDownloading(false);
      this.unlisten?.();
      this.unlisten = undefined;
    }
  }

  ngOnDestroy() {
    this.unlisten?.();
  }
}
