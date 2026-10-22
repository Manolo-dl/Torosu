import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { TASK_LABELS } from '@features/trainer';
import { FlnConverterParams, KeyConverterParams, KeyDensity, RateChangeParams, TrainerTaskType } from '@features/trainer/models/trainer.model';
import { TrainerStore } from '@features/trainer/stores/trainer.store';
import { TosuStore } from '@entities/tosu';
import { CdkDrag, CdkDragDrop, CdkDragHandle, CdkDropList } from '@angular/cdk/drag-drop';
import { TuiButton, TuiCheckbox, TuiDataList, TuiDropdown, TuiIcon, TuiInput, TuiNotificationDirective, TuiTitle } from '@taiga-ui/core';
import { TuiSegmented } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';
import { TrainerNoteLengthComponent } from '../trainer-note-length/trainer-note-length.component';

@Component({
  selector: 'app-trainer-task-pipeline',
  imports: [CdkDrag, CdkDropList, CdkDragHandle, FormsModule, TuiButton, TuiCardLarge, TuiCheckbox, TuiDataList, TuiDropdown, TuiIcon, TuiInput, TuiNotificationDirective, TuiSegmented, TuiTitle, TrainerNoteLengthComponent],
  templateUrl: './trainer-task-pipeline.component.html',
  styleUrl: './trainer-task-pipeline.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerTaskPipelineComponent {

  readonly trainer = inject(TrainerStore);
  readonly tosu = inject(TosuStore);

  taskLabels = TASK_LABELS;
  availableTasks: TrainerTaskType[] = ['RateChange', 'SVNormalizer', 'FlnConverter', 'KeyConverter'];
  readonly menuOpen = signal(false);

  availableTasksFiltered = computed(() => {
    const existingTypes = new Set(this.trainer.tasks().map(t => t.task.type));
    return this.availableTasks.filter(type => !existingTypes.has(type));
  });

  originalBpm = computed(() => this.tosu.data()?.beatmap?.stats?.bpm?.common ?? null);

  isMania = computed(() => this.tosu.data()?.beatmap?.mode?.number === 3);

  /** Key count of the current map: in mania it is the CS. */
  sourceKeys = computed(() => (this.isMania() ? Math.round(this.tosu.data()?.beatmap?.stats?.cs?.original ?? 0) || null : null));

  readonly keyOptions = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
  readonly densityOptions: { value: KeyDensity; label: string; hint: string }[] = [
    { value: 'none', label: 'None', hint: 'Only the notes of the original map.' },
    { value: 'dense', label: 'Some', hint: 'Widens chords into the key next to them, on the beats and chords.' },
    { value: 'moreDense', label: 'More', hint: 'Widens chords into the keys next to them, on almost every row.' },
    {
      value: 'full',
      label: 'Full',
      hint: 'Chords grow with the key count, like hand-made conversions: jumps become hands and hands become quints. Handstreams stay handstreams, and the map gets much harder.',
    },
  ];

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
      case 'KeyConverter': {
        const source = this.sourceKeys() ?? 4;
        this.trainer.addTask({
          type: 'KeyConverter',
          params: { targetKeys: source < 7 ? 7 : source < 10 ? 10 : 7, density: 'none', keepJacks: true },
        });
        break;
      }
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

  onKeyChange(id: string, params: KeyConverterParams, change: Partial<KeyConverterParams>) {
    this.trainer.updateTaskParams(id, { ...params, ...change });
  }

  densityIndex(density: KeyDensity): number {
    return this.densityOptions.findIndex(o => o.value === density);
  }

  onFlnChange(id: string, params: FlnConverterParams, change: Partial<FlnConverterParams>) {
    this.trainer.updateTaskParams(id, { ...params, ...change });
  }
}
