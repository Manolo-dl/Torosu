import { ChangeDetectionStrategy, Component } from '@angular/core';
import { TrainerBeatmapHeaderComponent, TrainerDifficultySlidersComponent, TrainerTaskPipelineComponent, TrainerActionsComponent } from "@features/trainer";
import { TuiTitle } from '@taiga-ui/core';

@Component({
  selector: 'app-trainer',
  imports: [TrainerBeatmapHeaderComponent, TrainerDifficultySlidersComponent, TrainerTaskPipelineComponent, TrainerActionsComponent, TuiTitle],
  templateUrl: './trainer-page.component.html',
  styleUrl: './trainer-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerPageComponent {}
