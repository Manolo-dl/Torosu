import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TrainerStore } from '@features/trainer/stores/trainer.store';
import { TuiButton } from '@taiga-ui/core';
import { TuiButtonLoading, TuiProgress } from '@taiga-ui/kit';

@Component({
  selector: 'app-trainer-actions',
  imports: [TuiButton, TuiButtonLoading, TuiProgress],
  templateUrl: './trainer-actions.component.html',
  styleUrl: './trainer-actions.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerActionsComponent {
  readonly trainer = inject(TrainerStore);
}
