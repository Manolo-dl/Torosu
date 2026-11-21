import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { translateSignal } from '@jsverse/transloco';
import { QUICK_RATES } from '../../models/trainer.model';

/** Rates one click away, the one set lit. */
@Component({
  selector: 'app-trainer-quick-rates',
  template: `
    @for (option of rates; track option) {
      <button type="button" [class.active]="rate() === option" [attr.aria-pressed]="rate() === option" (click)="rateChange.emit(option)">
        {{ option }}x
      </button>
    }
  `,
  styleUrl: './trainer-quick-rates.component.less',
  host: {
    role: 'group',
    '[attr.aria-label]': 'label()',
  },
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerQuickRatesComponent {
  readonly rate = input.required<number>();
  readonly rateChange = output<number>();
  readonly rates = QUICK_RATES;
  protected readonly label = translateSignal('trainer.rate.quick');
}
