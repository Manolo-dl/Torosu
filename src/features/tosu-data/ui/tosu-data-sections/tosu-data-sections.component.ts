import { Component, computed, inject, ChangeDetectionStrategy } from '@angular/core';
import { TosuStore } from '@entities/tosu';
import { TuiItem } from '@taiga-ui/cdk';
import { TuiExpand } from '@taiga-ui/core';
import { TuiAccordion, TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';
import { TosuFieldTreeComponent } from '../tosu-field-tree/tosu-field-tree.component';

@Component({
    selector: 'app-tosu-data-sections',
    imports: [TosuFieldTreeComponent, TuiAccordion, TuiBadge, TuiCardLarge, TuiExpand, TuiItem],
    templateUrl: './tosu-data-sections.component.html',
    styleUrl: './tosu-data-sections.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TosuDataSectionsComponent {
    store = inject(TosuStore);

    readonly sections = computed((): [string, unknown][] => {
        const data = this.store.data();
        return data ? Object.entries(data) : [];
    });

    typeLabel(v: unknown): string {
        if (Array.isArray(v)) return `[${v.length}]`;
        if (v !== null && typeof v === 'object') return `{${Object.keys(v).length}}`;
        return typeof v;
    }
}
