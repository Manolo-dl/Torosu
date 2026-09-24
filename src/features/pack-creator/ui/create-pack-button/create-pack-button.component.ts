import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { PackStore } from '@entities/pack';

@Component({
  selector: 'app-create-pack-button',
  imports: [],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './create-pack-button.component.html',
  styleUrl: './create-pack-button.component.css',
})
export class CreatePackButtonComponent {

  readonly store = inject(PackStore);

  async onCreate() {
    await this.store.createPack();
  }
}
