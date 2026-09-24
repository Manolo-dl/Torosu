import { Component } from '@angular/core';
import { TrainerBeatmapHeaderComponent, TrainerDifficultySlidersComponent, TrainerTaskPipelineComponent, TrainerActionsComponent } from "@features/trainer";

@Component({
  selector: 'app-trainer',
  imports: [TrainerBeatmapHeaderComponent, TrainerDifficultySlidersComponent, TrainerTaskPipelineComponent, TrainerActionsComponent],
  templateUrl: './trainer-page.component.html',
  styleUrl: './trainer-page.component.css',
})
export class TrainerPageComponent {}
