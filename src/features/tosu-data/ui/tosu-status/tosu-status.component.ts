import { Component, inject, ChangeDetectionStrategy } from '@angular/core';
import { TosuStore } from '@entities/tosu';
import { TuiButton } from '@taiga-ui/core';
import { TuiBadge, TuiStatus } from '@taiga-ui/kit';

@Component({
    selector: 'app-tosu-status',
    imports: [TuiBadge, TuiButton, TuiStatus],
    templateUrl: './tosu-status.component.html',
    styleUrl: './tosu-status.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TosuStatusComponent {
    store = inject(TosuStore);

    reconnect() {
        this.store.connect();
    }
}
