import { inject, Injectable, OnDestroy, signal } from '@angular/core';
import { DownloadQueueStore } from './download-queue.store';
import { UserStore } from './user.store';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { TranslocoService } from '@jsverse/transloco';
import { ConfigurationStore } from '@entities/configuration';
import { errorText } from '@shared/i18n';

/** How the downloads went (`DownloadSummary` in download.rs). */
interface DownloadSummary {
  succeeded: number;
  failed: number;
}

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
  private transloco = inject(TranslocoService);
  private config = inject(ConfigurationStore);
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
      const summary = await invoke<DownloadSummary>('start_downloads', {
        beatmapSetIds: pending.map(i => i.beatmapSetId),
      });
      if (this.config.config()?.downloader.notify) this.notifyDone(summary);
    } catch (e) {
      this.error.set(e ? errorText(this.transloco, e) : this.transloco.translate('downloader.queue.downloadFailed'));
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

  /** A notification of the system, seen with Torosu in the background. */
  private notifyDone({ succeeded, failed }: DownloadSummary) {
    invoke('show_notification', {
      title: this.transloco.translate('downloader.notification.title'),
      body: this.transloco.translate('downloader.notification.body', { succeeded, failed }),
    }).catch(e => console.warn('failed to show the notification', e));
  }

  ngOnDestroy() {
    this.unlisten?.();
  }
}
