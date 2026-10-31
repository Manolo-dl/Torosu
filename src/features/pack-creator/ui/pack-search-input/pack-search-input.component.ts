import { ChangeDetectionStrategy, Component, DestroyRef, inject } from '@angular/core';
import { PackStore } from '../../pack.store';
import { debounceTime, Subject } from 'rxjs';
import { TuiInput } from '@taiga-ui/core';

@Component({
  selector: 'app-pack-search-input',
  imports: [TuiInput],
  templateUrl: './pack-search-input.component.html',
  styleUrl: './pack-search-input.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PackSearchInputComponent {

  readonly store = inject(PackStore);
  private destroyRef = inject(DestroyRef);
  private inputChange = new Subject<string>();

  constructor() {
    this.inputChange
      .pipe(debounceTime(300))
      .subscribe(value => this.store.setSearchQuery(value));

    this.destroyRef.onDestroy(() => this.inputChange.complete());
  }

  onInput(event: Event) {
    const value = (event.target as HTMLInputElement).value;
    this.inputChange.next(value);
  }
}
