import { Component } from '@angular/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ButtonComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';

@Component({
  selector: 'app-window-controls',
  imports: [ButtonComponent, TranslocoPipe],
  templateUrl: './window-controls.component.html',
  styleUrl: './window-controls.component.less',
})
export class WindowControlsComponent {

  readonly window = getCurrentWindow();

  readonly controls = [
    { label: 'window.minimize', icon: 'minus', action: () => this.window.minimize() },
    { label: 'window.maximize', icon: 'square', action: () => this.window.toggleMaximize() },
    { label: 'window.close', icon: 'x', action: () => this.window.close() },
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
