import { Component, DestroyRef, inject } from '@angular/core';
import { PackStore } from '@entities/pack'; 
import { debounceTime, Subject } from 'rxjs';

@Component({
  selector: 'app-pack-search-input',
  imports: [],
  templateUrl: './pack-search-input.component.html',
  styleUrl: './pack-search-input.component.css',
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
