import { DestroyRef, Directive, ElementRef, EmbeddedViewRef, inject, input, TemplateRef, ViewContainerRef } from '@angular/core';

/** How long the pointer rests before it shows, and how long it takes to leave. */
const SHOW_DELAY_MS = 220;
const LEAVE_MS = 140;
/** Space between it and its element, and from the window's edges. */
const GAP = 8;
const MARGIN = 8;

/**
 * A card that shows over (or under) its element while the pointer rests on
 * it or it has the focus: `<span [appTooltip]="'Some text'">` or
 * `[appTooltip]="template" [tooltipContext]="value"`. It is put in <body>,
 * so nothing clips it; a template keeps the styles of the component it is
 * written in.
 */
@Directive({
    selector: '[appTooltip]',
    host: {
        '(mouseenter)': 'schedule()',
        '(mouseleave)': 'hide()',
        '(focusin)': 'show()',
        '(focusout)': 'hide()',
        '(keydown.escape)': 'hide()',
    },
})
export class TooltipDirective {
    readonly appTooltip = input.required<string | TemplateRef<{ $implicit: unknown }> | null>();
    readonly tooltipContext = input<unknown>();
    readonly tooltipPlacement = input<'top' | 'bottom'>('top');

    private readonly host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    private readonly views = inject(ViewContainerRef);
    private panel: HTMLElement | null = null;
    private view: EmbeddedViewRef<unknown> | null = null;
    private timer: ReturnType<typeof setTimeout> | undefined;

    constructor() {
        inject(DestroyRef).onDestroy(() => this.remove());
    }

    schedule() {
        clearTimeout(this.timer);
        this.timer = setTimeout(() => this.show(), SHOW_DELAY_MS);
    }

    show() {
        clearTimeout(this.timer);
        const content = this.appTooltip();
        if (!content) return;
        this.remove();

        const panel = document.createElement('div');
        panel.className = 'app-tooltip';
        panel.setAttribute('role', 'tooltip');
        if (typeof content === 'string') {
            panel.textContent = content;
        } else {
            this.view = this.views.createEmbeddedView(content, { $implicit: this.tooltipContext() });
            this.view.detectChanges();
            panel.append(...this.view.rootNodes);
        }
        document.body.append(panel);
        this.panel = panel;
        this.place(panel);
    }

    hide() {
        clearTimeout(this.timer);
        const panel = this.panel;
        if (!panel) return;
        panel.classList.add('leaving');
        this.timer = setTimeout(() => this.remove(), LEAVE_MS);
    }

    /** Over its element, or under it when there is no room over; inside the window. */
    private place(panel: HTMLElement) {
        const box = this.host.getBoundingClientRect();
        const { width, height } = panel.getBoundingClientRect();
        const roomAbove = box.top - GAP - MARGIN >= height;
        const roomBelow = innerHeight - box.bottom - GAP - MARGIN >= height;
        const above = this.tooltipPlacement() === 'top' ? roomAbove || !roomBelow : !roomBelow && roomAbove;
        const top = above ? box.top - GAP - height : box.bottom + GAP;
        const left = Math.min(Math.max(box.left + box.width / 2 - width / 2, MARGIN), innerWidth - width - MARGIN);
        panel.style.top = `${Math.max(MARGIN, top)}px`;
        panel.style.left = `${left}px`;
        panel.dataset['side'] = above ? 'top' : 'bottom';
        // Its arrow points at the element's middle, wherever the card ended up.
        panel.style.setProperty('--arrow-x', `${box.left + box.width / 2 - left}px`);
    }

    private remove() {
        this.view?.destroy();
        this.view = null;
        this.panel?.remove();
        this.panel = null;
    }
}
