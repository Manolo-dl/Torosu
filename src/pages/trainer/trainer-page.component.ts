import { ChangeDetectionStrategy, Component } from '@angular/core';
import { TrainerBeatmapHeaderComponent, TrainerDifficultySlidersComponent, TrainerTaskPipelineComponent, TrainerActionsComponent } from "@features/trainer";
import { PageHeaderComponent } from '@shared/ui';

@Component({
  selector: 'app-trainer',
  imports: [TrainerBeatmapHeaderComponent, TrainerDifficultySlidersComponent, TrainerTaskPipelineComponent, TrainerActionsComponent, PageHeaderComponent],
  templateUrl: './trainer-page.component.html',
  styleUrl: './trainer-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerPageComponent {}
