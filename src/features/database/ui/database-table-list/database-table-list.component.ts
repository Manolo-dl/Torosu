import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { DatabaseStore } from '@entities/database';
import { TuiCell, TuiIcon, TuiTitle } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-database-table-list',
  imports: [TuiBadge, TuiCardLarge, TuiCell, TuiIcon, TuiTitle],
  templateUrl: './database-table-list.component.html',
  styleUrl: './database-table-list.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DatabaseTableListComponent {
  readonly store = inject(DatabaseStore);
}
