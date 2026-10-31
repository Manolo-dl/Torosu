import { ChangeDetectionStrategy, Component } from '@angular/core';
import { LogViewerComponent } from '@features/logs';
import { PageHeaderComponent } from '@shared/ui';

@Component({
  selector: 'app-logs',
  imports: [LogViewerComponent, PageHeaderComponent],
  templateUrl: './logs-page.component.html',
  styleUrl: './logs-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LogsPageComponent {}
