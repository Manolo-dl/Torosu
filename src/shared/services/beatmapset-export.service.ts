import { Injectable } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';

@Injectable({ providedIn: 'root' })
export class BeatmapsetExportService {

  formatLine(beatmapsetId: number, format: 'urls' | 'ids'): string {
    return format === 'ids'
      ? `${beatmapsetId}`
      : `https://osu.ppy.sh/beatmapsets/${beatmapsetId}`;
  }

  async saveToFile(content: string, defaultPath = 'export.txt'): Promise<boolean> {
    const path = await save({
      filters: [{ name: 'Text', extensions: ['txt'] }],
      defaultPath,
    });
    if (!path) return false;
    await invoke('write_text_file', { path, content });
    return true;
  }
}
