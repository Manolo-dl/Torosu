import { Component, inject, ChangeDetectionStrategy } from '@angular/core';
import { TosuStore } from '@entities/tosu';

@Component({
    selector: 'app-tosu-status',
    imports: [],
    templateUrl: './tosu-status.component.html',
    styleUrl: './tosu-status.component.css',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TosuStatusComponent {
    store = inject(TosuStore);

    reconnect() {
        this.store.connect();
    }
}
