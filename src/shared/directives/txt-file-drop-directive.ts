import { Directive, OnDestroy, OnInit, output, signal } from '@angular/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { readTextFile } from '@tauri-apps/plugin-fs';

@Directive({
  selector: '[appTxtFileDrop]',
  exportAs: 'appTxtFileDrop',
})
export class TxtFileDropDirective implements OnInit, OnDestroy {
  readonly fileContent = output<string>();
  readonly isDragOver = signal(false);

  private unlisten?: () => void;

  async ngOnInit() {
    const win = getCurrentWindow();
    this.unlisten = await win.onDragDropEvent(async(event) => {

      if (event.payload.type === 'over') {
        this.isDragOver.set(true);
      } else if (event.payload.type === 'leave') {
        this.isDragOver.set(false);
      } else if (event.payload.type === 'drop') {
        this.isDragOver.set(false);
        
        for (const path of event.payload.paths) {
          if (path.endsWith('.txt')) {
            const content = await readTextFile(path);
            this.fileContent.emit(content);
          }
        }
      }
    });
  }

  ngOnDestroy() {
    this.unlisten?.();
  }
}
