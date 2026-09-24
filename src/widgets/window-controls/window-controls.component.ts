import { Component } from '@angular/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { FaIconComponent } from '@fortawesome/angular-fontawesome';
import { faMinimize, faMaximize, faXmark } from '@fortawesome/free-solid-svg-icons';
import { TuiButton, TuiHint } from '@taiga-ui/core';

@Component({
  selector: 'app-window-controls',
  imports: [FaIconComponent, TuiButton, TuiHint],
  templateUrl: './window-controls.component.html',
  styleUrl: './window-controls.component.less',
})
export class WindowControlsComponent {

  readonly window = getCurrentWindow();

  readonly controls = [
    { icon: faMinimize, action: () => this.window.minimize(), tooltip: 'Minimize' },
    { icon: faMaximize, action: () => this.window.toggleMaximize(), tooltip: 'Maximize' },
    { icon: faXmark, action: () => this.window.close(), tooltip: 'Close' }
  ]

  async onMouseDown(event: MouseEvent) {
    if (event.buttons === 1) {
      if (event.detail === 2) {
        // Doble clic: Maximizar / Restaurar
        await this.window.toggleMaximize();
      } else {
        // Clic simple: Arrastrar ventana
        await this.window.startDragging();
      }
    }
  }
}
