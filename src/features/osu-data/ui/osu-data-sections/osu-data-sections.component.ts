import { TranslocoPipe } from '@jsverse/transloco';
import { BadgeComponent, CardComponent, AccordionComponent, AccordionItemComponent, AccordionHeadingDirective, AccordionContentDirective } from '@shared/ui';
import { Component, computed, inject, ChangeDetectionStrategy } from '@angular/core';
import { OsuReaderStore } from '@entities/osu-reader';
import { OsuFieldTreeComponent } from '../osu-field-tree/osu-field-tree.component';

@Component({
    selector: 'app-osu-data-sections',
    imports: [TranslocoPipe, AccordionComponent, AccordionItemComponent, AccordionHeadingDirective, AccordionContentDirective, OsuFieldTreeComponent, BadgeComponent, CardComponent],
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
