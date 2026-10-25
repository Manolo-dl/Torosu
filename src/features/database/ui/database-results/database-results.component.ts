import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { ScrollingModule } from '@angular/cdk/scrolling';
import { DatabaseStore } from '@entities/database';
import { TuiLoader, TuiScrollControls, TuiScrollRef, TuiTitle } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-database-results',
  imports: [ScrollingModule, TuiBadge, TuiCardLarge, TuiLoader, TuiScrollControls, TuiScrollRef, TuiTitle],
  templateUrl: './database-results.component.html',
  styleUrl: './database-results.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DatabaseResultsComponent {
  readonly store = inject(DatabaseStore);

  formatValue(row: Record<string, unknown>, header: string): string {
    const value = row[header];
    if (value === null || value === undefined) return 'NULL';
    return String(value);
  }
}
