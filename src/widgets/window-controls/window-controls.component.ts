import { Component } from '@angular/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { TuiButton } from '@taiga-ui/core';
import { TranslocoPipe } from '@jsverse/transloco';

@Component({
  selector: 'app-window-controls',
  imports: [TuiButton, TranslocoPipe],
  templateUrl: './window-controls.component.html',
  styleUrl: './window-controls.component.less',
})
export class WindowControlsComponent {

  readonly window = getCurrentWindow();

  readonly controls = [
    { label: 'window.minimize', icon: '@tui.minimize-2', action: () => this.window.minimize() },
    { label: 'window.maximize', icon: '@tui.maximize-2', action: () => this.window.toggleMaximize() },
    { label: 'window.close', icon: '@tui.x', action: () => this.window.close() },
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
