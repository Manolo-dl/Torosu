import { Component, computed, inject, ChangeDetectionStrategy } from '@angular/core';
import { OsuReaderStore } from '@entities/osu-reader';
import { TuiItem } from '@taiga-ui/cdk';
import { TuiExpand } from '@taiga-ui/core';
import { TuiAccordion, TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';
import { OsuFieldTreeComponent } from '../osu-field-tree/osu-field-tree.component';

@Component({
    selector: 'app-osu-data-sections',
    imports: [OsuFieldTreeComponent, TuiAccordion, TuiBadge, TuiCardLarge, TuiExpand, TuiItem],
    templateUrl: './osu-data-sections.component.html',
    styleUrl: './osu-data-sections.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OsuDataSectionsComponent {
    store = inject(OsuReaderStore);

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
