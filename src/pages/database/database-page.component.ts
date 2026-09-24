import { Component } from '@angular/core';
import { DatabaseTableListComponent, DatabaseStructureComponent, DatabaseResultsComponent } from '@features/database';

@Component({
  selector: 'app-database',
  imports: [DatabaseTableListComponent, DatabaseStructureComponent, DatabaseResultsComponent],
  templateUrl: './database-page.component.html',
  styleUrl: './database-page.component.css',
})
export class DatabasePageComponent {}
