import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { DatabaseStore } from '@entities/database';

@Component({
  selector: 'app-database-structure',
  imports: [],
  templateUrl: './database-structure.component.html',
  styleUrl: './database-structure.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DatabaseStructureComponent {
  readonly store = inject(DatabaseStore);

  isExcluded(columnName: string): boolean {
    return this.store.excludedColumns().includes(columnName);
  }
}
