import { ChangeDetectionStrategy, Component, computed, inject, input, signal } from '@angular/core';
import { IconComponent } from '../icon/icon.component';

let nextId = 0;

/**
 * Items that open one at a time, in one box:
 * `<app-accordion><app-accordion-item heading="…">…</app-accordion-item></app-accordion>`.
 */
@Component({
    selector: 'app-accordion',
    template: '<ng-content />',
    styles: `
        :host {
            display: block;
            border: 1px solid var(--line-subtle);
            border-radius: var(--radius-lg);
            background: var(--surface-1);
            overflow: hidden;
        }
    `,
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AccordionComponent {
    /** The item open, null for none. */
    readonly open = signal<number | null>(null);
}

/** One item of an accordion, or alone: its heading opens and closes it. */
@Component({
    selector: 'app-accordion-item',
    imports: [IconComponent],
    template: `
        <button type="button" class="heading" [attr.aria-expanded]="expanded()" [attr.aria-controls]="panelId" (click)="toggle()">
            <span class="text">{{ heading() }}</span>
            <app-icon name="chevron-down" class="chevron" />
        </button>
        <div class="panel" [id]="panelId" role="region" [attr.inert]="expanded() ? null : ''">
            <div class="inner"><div class="content"><ng-content /></div></div>
        </div>
    `,
    styleUrl: './accordion-item.component.less',
    host: { '[class.expanded]': 'expanded()' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AccordionItemComponent {
    readonly heading = input.required<string>();

    private readonly group = inject(AccordionComponent, { optional: true });
    private readonly id = nextId++;
    readonly panelId = `accordion-panel-${this.id}`;
    /** Open, when it is not in a group. */
    private readonly alone = signal(false);

    readonly expanded = computed(() => (this.group ? this.group.open() === this.id : this.alone()));

    toggle() {
        if (this.group) this.group.open.set(this.expanded() ? null : this.id);
        else this.alone.update(open => !open);
    }
}
