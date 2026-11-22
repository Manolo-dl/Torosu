import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, input, model, output } from '@angular/core';
import { TuiError, TuiInput } from '@taiga-ui/core';
import { DisabledReason, ValidationError, WithOptionalFieldTree } from '@angular/forms/signals';

@Component({
  selector: 'app-stateful-input',
  imports: [TuiInput, TuiError, TranslocoPipe],
  templateUrl: './stateful-input.component.html',
  styleUrl: './stateful-input.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class StatefulInputComponent {
  value = model<string>('');

  /**
   * From the form (`[formField]`); the input only says it was left with
   * `touch`. Kept here as a model before, it stayed true after the form was
   * reset, and an emptied field showed it was required.
   */
  touched = input<boolean>(false);
  readonly touch = output<void>();

  disabled = input<boolean>(false);
  disabledReasons = input<readonly DisabledReason[]>([]);
  readonly = input<boolean>(false);
  hidden = input<boolean>(false);
  invalid = input<boolean>(false);
  errors = input<readonly WithOptionalFieldTree<ValidationError>[]>([]);

  /** The label, the placeholder and the errors' messages are translation keys. */
  label = input<string>('');
  id = input<string>('');
  type = input<string>('text');
  placeholder = input<string>('');
}
