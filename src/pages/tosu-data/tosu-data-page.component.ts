import { Component, ChangeDetectionStrategy } from '@angular/core';
import { TosuStatusComponent, TosuDataSectionsComponent } from '@features/tosu-data';
import { TuiTitle } from '@taiga-ui/core';

@Component({
    selector: 'app-tosu-data-page',
    imports: [TosuStatusComponent, TosuDataSectionsComponent, TuiTitle],
    templateUrl: './tosu-data-page.component.html',
    styleUrl: './tosu-data-page.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TosuDataPage {}
