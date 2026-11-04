import { inject, Injectable } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { beatmapsetUrl } from '@shared/utils';

@Injectable({ providedIn: 'root' })
export class BeatmapsetExportService {
  private readonly transloco = inject(TranslocoService);

  formatLine(beatmapsetId: number, format: 'urls' | 'ids'): string {
    return format === 'ids'
      ? `${beatmapsetId}`
      : beatmapsetUrl(beatmapsetId);
  }

  async saveToFile(content: string, defaultPath = 'export.txt'): Promise<boolean> {
    const path = await save({
      filters: [{ name: this.transloco.translate('export.textFiles'), extensions: ['txt'] }],
      defaultPath,
    });
    if (!path) return false;
    await invoke('write_text_file', { path, content });
    return true;
  }
}
