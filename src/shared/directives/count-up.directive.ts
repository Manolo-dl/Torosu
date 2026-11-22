import { DestroyRef, Directive, effect, ElementRef, inject, input, untracked } from '@angular/core';
import { AppearanceStore } from '@shared/stores';
import { LanguageService } from '@shared/i18n';

/** How long counting up takes. */
const DURATION_MS = 900;

/**
 * Writes a number counting up to it from the one it showed (0 at first), in
 * the language's digits: `<strong [appCountUp]="sets"></strong>`. At once
 * with reduced motion.
 */
@Directive({ selector: '[appCountUp]' })
export class CountUpDirective {
    readonly appCountUp = input.required<number>();

    private readonly element = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    private readonly appearance = inject(AppearanceStore);
    private readonly language = inject(LanguageService).current;
    private shown = 0;
    private frame = 0;

    constructor() {
        effect(() => {
            const target = this.appCountUp();
            const format = new Intl.NumberFormat(this.language());
            untracked(() => this.count(target, format));
        });
        inject(DestroyRef).onDestroy(() => cancelAnimationFrame(this.frame));
    }

    private count(target: number, format: Intl.NumberFormat) {
        cancelAnimationFrame(this.frame);
        const from = this.shown;
        const still = this.appearance.reducedMotion() || matchMedia('(prefers-reduced-motion: reduce)').matches;
        if (still || from === target) {
            this.write(target, format);
            return;
        }

        const start = performance.now();
        const step = (now: number) => {
            const t = Math.min((now - start) / DURATION_MS, 1);
            // Fast, then settling on it.
            const eased = 1 - Math.pow(1 - t, 3);
            this.write(Math.round(from + (target - from) * eased), format);
            if (t < 1) this.frame = requestAnimationFrame(step);
        };
        this.frame = requestAnimationFrame(step);
    }

    private write(value: number, format: Intl.NumberFormat) {
        this.shown = value;
        this.element.textContent = format.format(value);
    }
}
