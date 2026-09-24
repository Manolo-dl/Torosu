import { Component, computed, inject } from '@angular/core';
import { TASK_LABELS } from '@features/trainer';
import { TrainerTaskType } from '@features/trainer/models/trainer.model';
import { TrainerStore } from '@features/trainer/stores/trainer.store';
import { TosuStore } from '@entities/tosu';
import { CdkDrag, CdkDragDrop, CdkDragHandle, CdkDropList } from '@angular/cdk/drag-drop';

@Component({
  selector: 'app-trainer-task-pipeline',
  imports: [CdkDrag, CdkDropList, CdkDragHandle],
  templateUrl: './trainer-task-pipeline.component.html',
  styleUrl: './trainer-task-pipeline.component.css',
})
export class TrainerTaskPipelineComponent {

  readonly trainer = inject(TrainerStore);
  readonly tosu = inject(TosuStore);

  taskLabels = TASK_LABELS;
  availableTasks: TrainerTaskType[] = ['RateChange'];

  availableTasksFiltered = computed(() => {
    const existingTypes = new Set(this.trainer.tasks().map(t => t.task.type));
    return this.availableTasks.filter(type => !existingTypes.has(type));
  });

  originalBpm = computed(() => this.tosu.data()?.beatmap?.stats?.bpm?.common ?? null);

  newBpm(rate: number): number | null {
    const bpm = this.originalBpm();
    return bpm !== null ? Math.round(bpm * rate) : null;
  }

  onDrop(event: CdkDragDrop<unknown>) {
    this.trainer.reorderTasks(event.previousIndex, event.currentIndex);
  }

  addTask(type: TrainerTaskType | '') {
    switch (type) {
      case 'RateChange':
        this.trainer.addTask({ type: 'RateChange', params: { rate: 1.0, adjustPitch: false } });
        break;
    }
  }

  onRateInput(id: string, rateStr: string, adjustPitch: boolean) {
    const rate = parseFloat(rateStr);
    if (isNaN(rate) || rate <= 0) return;
    this.trainer.updateTaskParams(id, { rate, adjustPitch });
  }

  onBpmInput(id: string, bpmStr: string, adjustPitch: boolean) {
    const bpm = this.originalBpm();
    if (bpm === null) return;
    const newBpm = parseFloat(bpmStr);
    if (isNaN(newBpm) || newBpm <= 0) return;
    const rate = Math.round((newBpm / bpm) * 100) / 100;
    this.trainer.updateTaskParams(id, { rate, adjustPitch });
  }

  onMirrorChange(id: string, axis: 'horizontal' | 'vertical') {
    this.trainer.updateTaskParams(id, { axis });
  }
}
