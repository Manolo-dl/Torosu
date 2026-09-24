import { ChangeDetectionStrategy, Component } from '@angular/core';
import { LogViewerComponent } from '@features/logs';

@Component({
  selector: 'app-logs',
  imports: [LogViewerComponent],
  templateUrl: './logs-page.component.html',
  styleUrl: './logs-page.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LogsPageComponent {}
