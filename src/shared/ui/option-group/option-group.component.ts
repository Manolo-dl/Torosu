import {
  afterRenderEffect,
  booleanAttribute,
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  inject,
  input,
  output,
  viewChild,
  viewChildren,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';

export interface OptionGroupItem {
  label: string;
  /** `label` is a translation key. */
  translate?: boolean;
  disabled?: boolean;
}

/**
 * One option out of a few, the one picked on a fill that slides to it. Unlike
 * Taiga's segmented control the options wrap onto more lines when they don't
 * fit, and the fill follows them there: on a narrow window that one cut them
 * off or squeezed them together.
 */
@Component({
  selector: 'app-option-group',
  imports: [TranslocoPipe],
  template: `
    <span class="indicator" aria-hidden="true" #indicator></span>
    @for (option of options(); track $index) {
      <button
        #option
        role="radio"
        type="button"
        [attr.aria-checked]="$index === active()"
        [class.active]="$index === active()"
        [disabled]="option.disabled"
        (click)="pick($index)"
      >
        {{ option.translate ? (option.label | transloco) : option.label }}
      </button>
    }
  `,
  styleUrl: './option-group.component.less',
  host: { role: 'radiogroup', '[attr.data-size]': 'size()', '[class.stretch]': 'stretch()' },
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OptionGroupComponent {
  readonly options = input.required<readonly OptionGroupItem[]>();
  /** Index of the option picked. */
  readonly active = input.required<number>();
  readonly size = input<'s' | 'm'>('m');
  /** As wide as where it is, its options sharing the width. */
  readonly stretch = input(false, { transform: booleanAttribute });
  readonly activeChange = output<number>();

  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly indicator = viewChild.required<ElementRef<HTMLElement>>('indicator');
  private readonly buttons = viewChildren<ElementRef<HTMLButtonElement>>('option');

  constructor() {
    // The fill goes where the option picked is, again when the options wrap
    // differently (the window or the labels changed size).
    afterRenderEffect(() => {
      this.active();
      this.buttons();
      this.place();
    });

    const observer = new ResizeObserver(() => this.place());
    observer.observe(this.host.nativeElement);
    inject(DestroyRef).onDestroy(() => observer.disconnect());
  }

  pick(index: number) {
    if (index !== this.active()) this.activeChange.emit(index);
  }

  private place() {
    const indicator = this.indicator().nativeElement;
    const button = this.buttons()[this.active()]?.nativeElement;
    if (!button) {
      indicator.style.opacity = '0';
      return;
    }
    indicator.style.opacity = '1';
    indicator.style.inlineSize = `${button.offsetWidth}px`;
    indicator.style.blockSize = `${button.offsetHeight}px`;
    indicator.style.transform = `translate(${button.offsetLeft}px, ${button.offsetTop}px)`;
    // Placed once without moving: from then on it slides.
    if (!indicator.classList.contains('ready')) {
      requestAnimationFrame(() => indicator.classList.add('ready'));
    }
  }
}
