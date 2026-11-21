import { ButtonComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TrainerStore } from '../../stores/trainer.store';
import { TuiProgress } from '@taiga-ui/kit';

@Component({
  selector: 'app-trainer-actions',
  imports: [TuiProgress, TranslocoPipe, ButtonComponent],
  templateUrl: './trainer-actions.component.html',
  styleUrl: './trainer-actions.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerActionsComponent {
  readonly trainer = inject(TrainerStore);
}
