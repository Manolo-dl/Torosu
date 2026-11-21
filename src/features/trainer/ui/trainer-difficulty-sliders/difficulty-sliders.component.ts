import { ButtonComponent, CardComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { DifficultyKey } from '../../models/trainer.model';
import { TrainerStore } from '../../stores/trainer.store';
import { TuiSlider, TuiTitle } from '@taiga-ui/core';

@Component({
  selector: 'app-trainer-difficulty-sliders',
  imports: [DecimalPipe, FormsModule, TuiSlider, TuiTitle, TranslocoPipe, ButtonComponent, CardComponent],
  templateUrl: './difficulty-sliders.component.html',
  styleUrl: './difficulty-sliders.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerDifficultySlidersComponent {
  readonly trainer = inject(TrainerStore);
  readonly keys: DifficultyKey[] = ['hp', 'cs', 'ar', 'od'];
}
