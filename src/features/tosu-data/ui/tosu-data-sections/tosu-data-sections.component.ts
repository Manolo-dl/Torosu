import { Component, inject, ChangeDetectionStrategy } from '@angular/core';
import { TosuStore } from '@entities/tosu';

@Component({
    selector: 'app-tosu-data-sections',
    imports: [],
    templateUrl: './tosu-data-sections.component.html',
    styleUrl: './tosu-data-sections.component.css',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TosuDataSectionsComponent {
    store = inject(TosuStore);

    sections(): [string, unknown][] {
        const data = this.store.data();
        if (!data) return [];
        return Object.entries(data);
    }

    format(v: unknown): string {
        return JSON.stringify(v, null, 2);
    }

    isPrimitive(v: unknown): boolean {
        return v === null || typeof v !== 'object';
    }
}
