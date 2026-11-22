import { ButtonComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { PackStore } from '../../pack.store';

@Component({
  selector: 'app-create-pack-button',
  imports: [TranslocoPipe, ButtonComponent],
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
