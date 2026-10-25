import { ChangeDetectionStrategy, Component } from '@angular/core';
import { LogViewerComponent } from '@features/logs';
import { TuiTitle } from '@taiga-ui/core';

@Component({
  selector: 'app-logs',
  imports: [LogViewerComponent, TuiTitle],
  templateUrl: './logs-page.component.html',
  styleUrl: './logs-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LogsPageComponent {}
