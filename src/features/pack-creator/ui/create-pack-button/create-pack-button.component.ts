import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { PackStore } from '@entities/pack';
import { TuiButton } from '@taiga-ui/core';
import { TuiButtonLoading } from '@taiga-ui/kit';

@Component({
  selector: 'app-create-pack-button',
  imports: [TuiButton, TuiButtonLoading],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './create-pack-button.component.html',
  styleUrl: './create-pack-button.component.less',
})
export class CreatePackButtonComponent {

  readonly store = inject(PackStore);

  async onCreate() {
    await this.store.createPack();
  }
}
