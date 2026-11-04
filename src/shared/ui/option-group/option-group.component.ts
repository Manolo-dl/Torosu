import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';

export interface OptionGroupItem {
  label: string;
  /** `label` is a translation key. */
  translate?: boolean;
  disabled?: boolean;
}

/**
 * One option out of a few, like Taiga's segmented control but wrapping onto
 * more lines when they don't fit: that one slides its highlight along a single
 * row, so on a narrow window its options were cut off or squeezed together.
 */
@Component({
  selector: 'app-option-group',
  imports: [TranslocoPipe],
  template: `
    @for (option of options(); track $index) {
      <button
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
  host: { role: 'radiogroup' },
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OptionGroupComponent {
  readonly options = input.required<readonly OptionGroupItem[]>();
  /** Index of the option picked. */
  readonly active = input.required<number>();
  readonly activeChange = output<number>();

  pick(index: number) {
    if (index !== this.active()) this.activeChange.emit(index);
  }
}
