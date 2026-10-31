import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TrainerBeatmapHeaderComponent, TrainerDifficultySlidersComponent, TrainerTaskPipelineComponent, TrainerActionsComponent, TrainerPresetsComponent, TrainerStore } from "@features/trainer";
import { TuiButton } from '@taiga-ui/core';
import { PageHeaderComponent } from '@shared/ui';

@Component({
  selector: 'app-trainer',
  imports: [TrainerBeatmapHeaderComponent, TrainerDifficultySlidersComponent, TrainerTaskPipelineComponent, TrainerActionsComponent, TrainerPresetsComponent, PageHeaderComponent, TuiButton],
  templateUrl: './trainer-page.component.html',
  styleUrl: './trainer-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerPageComponent {
  readonly trainer = inject(TrainerStore);
}
