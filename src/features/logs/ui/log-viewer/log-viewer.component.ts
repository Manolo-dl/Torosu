import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { LogStore } from '@entities/log';
import { LogColumnComponent } from '../log-column/log-column.component';

@Component({
  selector: 'app-log-viewer',
  imports: [LogColumnComponent],
  templateUrl: './log-viewer.component.html',
  styleUrl: './log-viewer.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LogViewerComponent {
  readonly store = inject(LogStore);

  async reload() {
    await this.store.load();
  }
}
