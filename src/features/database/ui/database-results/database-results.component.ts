import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { DatabaseStore } from '@entities/database';

@Component({
  selector: 'app-database-results',
  imports: [],
  templateUrl: './database-results.component.html',
  styleUrl: './database-results.component.css',
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
