import { DestroyRef, ElementRef, Signal, inject, signal } from '@angular/core';

/** Narrower than this, a chart would squeeze its labels together. */
const MIN_WIDTH = 280;

/**
 * The width of the host element in pixels, kept up to date. Charts draw at it,
 * so their text keeps its size however wide the window is. Call it in a
 * constructor or field initialiser.
 */
export function hostWidth(initial = 640): Signal<number> {
    const host = inject<ElementRef<HTMLElement>>(ElementRef);
    const width = signal(initial);
    const observer = new ResizeObserver(([entry]) => {
        width.set(Math.max(MIN_WIDTH, Math.floor(entry.contentRect.width)));
    });
    observer.observe(host.nativeElement);
    inject(DestroyRef).onDestroy(() => observer.disconnect());
    return width.asReadonly();
}
