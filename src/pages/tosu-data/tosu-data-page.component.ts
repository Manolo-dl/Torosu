import { Component, ChangeDetectionStrategy } from '@angular/core';
import { TosuStatusComponent, TosuDataSectionsComponent } from '@features/tosu-data';

@Component({
    selector: 'app-tosu-data-page',
    imports: [TosuStatusComponent, TosuDataSectionsComponent],
    templateUrl: './tosu-data-page.component.html',
    styleUrl: './tosu-data-page.component.css',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TosuDataPage {}
