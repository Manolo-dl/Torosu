import { TranslocoPipe } from '@jsverse/transloco';
import { ButtonComponent, BadgeComponent } from '@shared/ui';
import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { LogStore } from '@entities/log';
import { LogColumnComponent } from '../log-column/log-column.component';

@Component({
  selector: 'app-log-viewer',
  imports: [TranslocoPipe, LogColumnComponent, ButtonComponent, BadgeComponent],
  templateUrl: './log-viewer.component.html',
  styleUrl: './log-viewer.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LogViewerComponent {
  readonly store = inject(LogStore);
  readonly levels = ['DEBUG', 'INFO', 'WARN', 'ERROR'];
  readonly reloading = signal(false);

  async reload() {
    this.reloading.set(true);
    try {
      await this.store.load();
    } finally {
      this.reloading.set(false);
    }
  }
}
