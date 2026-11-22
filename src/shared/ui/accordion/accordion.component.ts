import { NgTemplateOutlet } from '@angular/common';
import {
    booleanAttribute,
    ChangeDetectionStrategy,
    Component,
    computed,
    contentChild,
    DestroyRef,
    Directive,
    effect,
    inject,
    input,
    linkedSignal,
    OnInit,
    signal,
    TemplateRef,
} from '@angular/core';
import { IconComponent } from '../icon/icon.component';

let nextId = 0;

/** Longer than closing takes (--duration-3), so lazy content leaves once out of sight. */
const CLOSE_MS = 400;

/**
 * Items in one box, opening one at a time unless `closeOthers` is false:
 * `<app-accordion><app-accordion-item heading="…">…</app-accordion-item></app-accordion>`.
 * `size="s"` is the compact one, for lists.
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
    host: { '[attr.data-size]': 'size()' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AccordionComponent {
    readonly closeOthers = input(true, { transform: booleanAttribute });
    readonly size = input<'s' | 'm'>('m');
    /** The items open. */
    readonly open = signal<ReadonlySet<number>>(new Set());

    toggle(id: number) {
        const open = new Set(this.closeOthers() ? [...this.open()].filter(other => other === id) : this.open());
        if (open.has(id)) open.delete(id);
        else open.add(id);
        this.open.set(open);
    }
}

/** What goes in the heading instead of `heading`: `<span appAccordionHeading>…</span>`. */
@Directive({ selector: '[appAccordionHeading]' })
export class AccordionHeadingDirective {}

/**
 * Content made only while the item is open, for long or live ones:
 * `<ng-template appAccordionContent>…</ng-template>`.
 */
@Directive({ selector: 'ng-template[appAccordionContent]' })
export class AccordionContentDirective {
    readonly template = inject(TemplateRef);
}

/** One item of an accordion, or alone: its heading opens and closes it. */
@Component({
    selector: 'app-accordion-item',
    imports: [IconComponent, NgTemplateOutlet],
    template: `
        <button type="button" class="heading" [attr.aria-expanded]="expanded()" [attr.aria-controls]="panelId" (click)="toggle()">
            <span class="text">
                <ng-content select="[appAccordionHeading]" />
                {{ heading() }}
            </span>
            <app-icon name="chevron-down" class="chevron" />
        </button>
        <div class="panel" [id]="panelId" role="region" [attr.inert]="expanded() ? null : ''">
            <div class="inner">
                <div class="content">
                    <ng-content />
                    @if (lazy(); as lazy) {
                        @if (rendered()) {
                            <ng-container [ngTemplateOutlet]="lazy.template" />
                        }
                    }
                </div>
            </div>
        </div>
    `,
    styleUrl: './accordion-item.component.less',
    host: { '[class.expanded]': 'expanded()' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AccordionItemComponent implements OnInit {
    readonly heading = input('');
    /** Open from the start. */
    readonly initiallyOpen = input(false, { transform: booleanAttribute, alias: 'open' });

    readonly lazy = contentChild(AccordionContentDirective);

    private readonly group = inject(AccordionComponent, { optional: true });
    private readonly id = nextId++;
    readonly panelId = `accordion-panel-${this.id}`;
    /** Open, when it is not in a group. */
    private readonly alone = linkedSignal(() => this.initiallyOpen());

    readonly expanded = computed(() => (this.group ? this.group.open().has(this.id) : this.alone()));
    /** Its lazy content is made: while open, and until it has finished closing. */
    readonly rendered = signal(false);

    constructor() {
        let closing: ReturnType<typeof setTimeout> | undefined;
        effect(() => {
            clearTimeout(closing);
            if (this.expanded()) this.rendered.set(true);
            else closing = setTimeout(() => this.rendered.set(false), CLOSE_MS);
        });
        inject(DestroyRef).onDestroy(() => clearTimeout(closing));
    }

    ngOnInit() {
        if (this.initiallyOpen() && this.group) this.group.toggle(this.id);
    }

    toggle() {
        if (this.group) this.group.toggle(this.id);
        else this.alone.update(open => !open);
    }
}
