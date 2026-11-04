import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TrainerStore } from '../../stores/trainer.store';
import { TuiButton } from '@taiga-ui/core';
import { TuiButtonLoading, TuiProgress } from '@taiga-ui/kit';

@Component({
  selector: 'app-trainer-actions',
  imports: [TuiButton, TuiButtonLoading, TuiProgress, TranslocoPipe],
  templateUrl: './trainer-actions.component.html',
  styleUrl: './trainer-actions.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerActionsComponent {
  readonly trainer = inject(TrainerStore);
}
