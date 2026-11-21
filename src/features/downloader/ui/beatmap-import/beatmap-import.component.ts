import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { OsuDbStore } from '@entities/osu-db';
import { NotifyService } from '@shared/services';
import { ButtonComponent, IconComponent, OptionGroupComponent, OptionGroupItem, TxtDropzoneComponent } from '@shared/ui';
import { DownloadQueueStore } from '../../download-queue.store';
import { TxtParserService } from '../../txt-parser.service';

/**
 * The maps to download: from a .txt file, or links (or set IDs) pasted in.
 * Says how many went into the queue, and how many were there already.
 */
@Component({
  selector: 'app-beatmap-import',
  imports: [ButtonComponent, IconComponent, OptionGroupComponent, TranslocoPipe, TxtDropzoneComponent],
  templateUrl: './beatmap-import.component.html',
  styleUrl: './beatmap-import.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapImportComponent {
  private readonly downloadQueue = inject(DownloadQueueStore);
  private readonly parser = inject(TxtParserService);
  private readonly osuDb = inject(OsuDbStore);
  private readonly notify = inject(NotifyService);
  private readonly transloco = inject(TranslocoService);

  readonly modes: OptionGroupItem[] = [
    { label: 'downloader.importer.file', icon: 'file-text', translate: true },
    { label: 'downloader.importer.paste', icon: 'clipboard-paste', translate: true },
  ];
  readonly mode = signal(0);

  /** What is pasted, and the sets found in it as it is typed. */
  readonly text = signal('');
  readonly found = computed(() => new Set(this.parser.parse(this.text())).size);

  /** Adds the sets in `content` to the queue, saying how many; `clear` empties what was pasted. */
  add(content: string, clear = false) {
    const ids = [...new Set(this.parser.parse(content))];
    if (ids.length === 0) {
      this.notify.error(this.transloco.translate('downloader.importer.nothing'));
      return;
    }
    const queued = new Set(this.downloadQueue.queue().map(item => item.beatmapSetId));
    const owned = this.osuDb.localSubmittedSetIds();
    const fresh = ids.filter(id => !queued.has(id) && !owned.has(id)).length;
    const skipped = ids.length - fresh;

    this.downloadQueue.addToQueue(ids);
    this.notify.success(
      skipped > 0
        ? this.transloco.translate('downloader.importer.addedSkipped', { count: fresh, skipped })
        : this.transloco.translate('downloader.importer.added', { count: fresh }),
    );
    if (clear) this.text.set('');
  }
}
