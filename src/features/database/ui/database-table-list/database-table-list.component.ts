import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { DatabaseStore } from '@entities/database';

@Component({
  selector: 'app-database-table-list',
  imports: [],
  templateUrl: './database-table-list.component.html',
  styleUrl: './database-table-list.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DatabaseTableListComponent {
  readonly store = inject(DatabaseStore);
}
