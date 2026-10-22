import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { TASK_LABELS } from '@features/trainer';
import { FlnConverterParams, RateChangeParams, TrainerTaskType } from '@features/trainer/models/trainer.model';
import { TrainerStore } from '@features/trainer/stores/trainer.store';
import { TosuStore } from '@entities/tosu';
import { CdkDrag, CdkDragDrop, CdkDragHandle, CdkDropList } from '@angular/cdk/drag-drop';
import { TuiButton, TuiCheckbox, TuiDataList, TuiDropdown, TuiIcon, TuiInput, TuiNotificationDirective, TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';
import { TrainerNoteLengthComponent } from '../trainer-note-length/trainer-note-length.component';

@Component({
  selector: 'app-trainer-task-pipeline',
  imports: [CdkDrag, CdkDropList, CdkDragHandle, FormsModule, TuiButton, TuiCardLarge, TuiCheckbox, TuiDataList, TuiDropdown, TuiIcon, TuiInput, TuiNotificationDirective, TuiTitle, TrainerNoteLengthComponent],
  templateUrl: './trainer-task-pipeline.component.html',
  styleUrl: './trainer-task-pipeline.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerTaskPipelineComponent {

  readonly trainer = inject(TrainerStore);
  readonly tosu = inject(TosuStore);

  taskLabels = TASK_LABELS;
  availableTasks: TrainerTaskType[] = ['RateChange', 'SVNormalizer', 'FlnConverter'];
  readonly menuOpen = signal(false);

  availableTasksFiltered = computed(() => {
    const existingTypes = new Set(this.trainer.tasks().map(t => t.task.type));
    return this.availableTasks.filter(type => !existingTypes.has(type));
  });

  originalBpm = computed(() => this.tosu.data()?.beatmap?.stats?.bpm?.common ?? null);

  isMania = computed(() => this.tosu.data()?.beatmap?.mode?.number === 3);

  newBpm(rate: number): number | null {
    const bpm = this.originalBpm();
    return bpm !== null ? Math.round(bpm * rate) : null;
  }

  onDrop(event: CdkDragDrop<unknown>) {
    this.trainer.reorderTasks(event.previousIndex, event.currentIndex);
  }

  addTask(type: TrainerTaskType) {
    this.menuOpen.set(false);
    switch (type) {
      case 'RateChange':
        this.trainer.addTask({ type: 'RateChange', params: { rate: 1.0, adjustPitch: false, scaleStoryboard: false, scaleVideo: false } });
        break;
      case 'SVNormalizer':
        this.trainer.addTask({ type: 'SVNormalizer', params: {} });
        break;
      case 'FlnConverter':
        this.trainer.addTask({
          type: 'FlnConverter',
          params: { gap: { unit: 'beat', value: 4 }, minLength: { unit: 'beat', value: 4 }, reworkExistingLn: true },
        });
        break;
    }
  }

  onRateInput(id: string, rateStr: string, params: RateChangeParams) {
    const rate = parseFloat(rateStr);
    if (isNaN(rate) || rate <= 0) return;
    this.trainer.updateTaskParams(id, { ...params, rate });
  }

  onBpmInput(id: string, bpmStr: string, params: RateChangeParams) {
    const bpm = this.originalBpm();
    if (bpm === null) return;
    const newBpm = parseFloat(bpmStr);
    if (isNaN(newBpm) || newBpm <= 0) return;
    const rate = Math.round((newBpm / bpm) * 100) / 100;
    this.trainer.updateTaskParams(id, { ...params, rate });
  }

  onRateOptionChange(id: string, params: RateChangeParams, change: Partial<RateChangeParams>) {
    this.trainer.updateTaskParams(id, { ...params, ...change });
  }

  onFlnChange(id: string, params: FlnConverterParams, change: Partial<FlnConverterParams>) {
    this.trainer.updateTaskParams(id, { ...params, ...change });
  }
}
