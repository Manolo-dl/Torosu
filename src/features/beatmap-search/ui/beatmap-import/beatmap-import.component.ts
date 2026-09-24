import { ChangeDetectionStrategy, Component, inject, OnDestroy, OnInit, signal } from '@angular/core';
import { BeatmapStore } from '@entities/beatmap';
import { TxtParserService } from '@features/beatmap-search/txt-parser.service';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { readTextFile } from '@tauri-apps/plugin-fs';

@Component({
  selector: 'app-beatmap-import',
  imports: [],
  templateUrl: './beatmap-import.component.html',
  styleUrl: './beatmap-import.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapImportComponent implements OnInit, OnDestroy {
  private beatmapStore = inject(BeatmapStore);
  private parser = inject(TxtParserService);
  private unlisten?: () => void;

  isDragOver = signal(false);

  async ngOnInit() {
    const win = getCurrentWindow();
    this.unlisten = await win.onDragDropEvent(async (event) => {
      if (event.payload.type === 'over') {
        this.isDragOver.set(true);
      } else if (event.payload.type === 'leave') {
        this.isDragOver.set(false);
      } else if (event.payload.type === 'drop') {
        this.isDragOver.set(false);
        for (const path of event.payload.paths) {
          if (path.endsWith('.txt')) {
            const content = await readTextFile(path);
            const items = this.parser.parse(content);
            this.beatmapStore.addToQueue(items);
          }
        }
      }
    });
  }

  ngOnDestroy() {
    this.unlisten?.();
  }

  onFileSelect(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (file) this.readFile(file);
    input.value = '';
  }

  private readFile(file: File) {
    const reader = new FileReader();
    reader.onload = (e) => {
      const content = e.target?.result as string;
      const items = this.parser.parse(content);
      this.beatmapStore.addToQueue(items);
    };
    reader.readAsText(file);
  }
}
