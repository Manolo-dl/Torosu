import { ChangeDetectionStrategy, Component, input, model } from '@angular/core';
import { DisabledReason, ValidationError, WithOptionalFieldTree } from '@angular/forms/signals';

@Component({
  selector: 'app-stateful-input',
  imports: [],
  templateUrl: './stateful-input.component.html',
  styleUrl: './stateful-input.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class StatefulInputComponent {
  value = model<string>('');

  touched = model<boolean>(false);

  disabled = input<boolean>(false);
  disabledReasons = input<readonly DisabledReason[]>([]);
  readonly = input<boolean>(false);
  hidden = input<boolean>(false);
  invalid = input<boolean>(false);
  errors = input<readonly WithOptionalFieldTree<ValidationError>[]>([]);

  label = input<string>('');
  id = input<string>('');
  type = input<string>('text');
  placeholder = input<string>('');
}
