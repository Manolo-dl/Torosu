import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { DifficultyKey } from '../../models/trainer.model';
import { TrainerStore } from '../../stores/trainer.store';
import { TuiButton, TuiSlider, TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-trainer-difficulty-sliders',
  imports: [DecimalPipe, FormsModule, TuiButton, TuiCardLarge, TuiSlider, TuiTitle],
  templateUrl: './difficulty-sliders.component.html',
  styleUrl: './difficulty-sliders.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerDifficultySlidersComponent {
  readonly trainer = inject(TrainerStore);
  readonly keys: DifficultyKey[] = ['hp', 'cs', 'ar', 'od'];
}
