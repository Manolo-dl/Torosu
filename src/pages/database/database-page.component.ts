import { ChangeDetectionStrategy, Component } from '@angular/core';
import { DatabaseTableListComponent, DatabaseStructureComponent, DatabaseResultsComponent } from '@features/database';
import { TuiTitle } from '@taiga-ui/core';

@Component({
  selector: 'app-database',
  imports: [DatabaseTableListComponent, DatabaseStructureComponent, DatabaseResultsComponent, TuiTitle],
  templateUrl: './database-page.component.html',
  styleUrl: './database-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DatabasePageComponent {}
