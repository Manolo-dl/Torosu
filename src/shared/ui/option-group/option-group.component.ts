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
import { IconComponent } from '../icon/icon.component';

export interface OptionGroupItem {
  label: string;
  /** Shown before the label, for the `nav` variant. */
  icon?: string;
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
  imports: [IconComponent, TranslocoPipe],
  template: `
    <span class="indicator" aria-hidden="true" #indicator></span>
    @for (option of options(); track $index) {
      <button
        #option
        type="button"
        [attr.role]="variant() === 'pills' ? 'radio' : 'tab'"
        [attr.title]="variant() === 'nav' ? (option.translate ? (option.label | transloco) : option.label) : null"
        [attr.aria-checked]="variant() === 'pills' ? $index === active() : null"
        [attr.aria-selected]="variant() === 'pills' ? null : $index === active()"
        [class.active]="$index === active()"
        [disabled]="option.disabled"
        (click)="pick($index)"
      >
        @if (option.icon) {
          <app-icon [name]="option.icon" />
        }
        <span class="label">{{ option.translate ? (option.label | transloco) : option.label }}</span>
      </button>
    }
  `,
  styleUrl: './option-group.component.less',
  host: {
    '[attr.role]': 'variant() === "pills" ? "radiogroup" : "tablist"',
    '[attr.aria-orientation]': 'variant() === "nav" ? "vertical" : null',
    '[attr.data-size]': 'size()',
    '[attr.data-variant]': 'variant()',
    '[class.stretch]': 'stretch()',
  },
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OptionGroupComponent {
  readonly options = input.required<readonly OptionGroupItem[]>();
  /** Index of the option picked. */
  readonly active = input.required<number>();
  readonly size = input<'s' | 'm'>('m');
  /** `tabs`: a row of tabs, the one open underlined; `nav`: a column of them, with icons. */
  readonly variant = input<'pills' | 'tabs' | 'nav'>('pills');
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
    // Tabs: a line under the tab rather than a fill behind it.
    const line = this.variant() === 'tabs' ? 2 : 0;
    indicator.style.blockSize = `${line || button.offsetHeight}px`;
    const top = line ? button.offsetTop + button.offsetHeight - line : button.offsetTop;
    indicator.style.transform = `translate(${button.offsetLeft}px, ${top}px)`;
    // Placed once without moving: from then on it slides.
    if (!indicator.classList.contains('ready')) {
      requestAnimationFrame(() => indicator.classList.add('ready'));
    }
  }
}
