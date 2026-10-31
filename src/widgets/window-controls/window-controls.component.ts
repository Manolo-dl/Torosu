import { Component } from '@angular/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { TuiButton } from '@taiga-ui/core';

@Component({
  selector: 'app-window-controls',
  imports: [TuiButton],
  templateUrl: './window-controls.component.html',
  styleUrl: './window-controls.component.less',
})
export class WindowControlsComponent {

  readonly window = getCurrentWindow();

  readonly controls = [
    { label: 'Minimize', icon: '@tui.minimize-2', action: () => this.window.minimize() },
    { label: 'Maximize', icon: '@tui.maximize-2', action: () => this.window.toggleMaximize() },
    { label: 'Close', icon: '@tui.x', action: () => this.window.close() },
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
