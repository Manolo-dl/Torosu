import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { DatabaseStore } from '@entities/database';
import { TuiIcon, TuiTitle } from '@taiga-ui/core';
import { TuiBadge, TuiChip } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-database-structure',
  imports: [TuiBadge, TuiCardLarge, TuiChip, TuiIcon, TuiTitle],
  templateUrl: './database-structure.component.html',
  styleUrl: './database-structure.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DatabaseStructureComponent {
  readonly store = inject(DatabaseStore);

  isExcluded(columnName: string): boolean {
    return this.store.excludedColumns().includes(columnName);
  }
}
