import { ChangeDetectionStrategy, Component, computed, inject, input, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import {
  FlnConverterParams,
  KeyConverterParams,
  KeyDensity,
  PLAYSTYLES,
  RateChangeParams,
  TASK_LABELS,
  TrainerConvert,
  rateChangeTask,
  TrainerTaskType,
  TASK_MODES,
  taskSupportsMode,
} from '../../models/trainer.model';
import { TrainerStore } from '../../stores/trainer.store';
import { ConfigurationStore } from '@entities/configuration';
import { OsuReaderStore } from '@entities/osu-reader';
import { CdkDrag, CdkDragDrop, CdkDragHandle, CdkDropList } from '@angular/cdk/drag-drop';
import { TuiButton, TuiCheckbox, TuiDataList, TuiDropdown, TuiIcon, TuiInput, TuiNotificationDirective, TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';
import { TrainerNoteLengthComponent } from '../trainer-note-length/trainer-note-length.component';
import { OptionGroupComponent, OptionGroupItem } from '@shared/ui';

interface DensityOption {
  value: KeyDensity;
  label: string;
  hint: string;
}

/** The tasks of one convert. */
@Component({
  selector: 'app-trainer-task-pipeline',
  imports: [CdkDrag, CdkDropList, CdkDragHandle, FormsModule, TuiButton, TuiCardLarge, TuiCheckbox, TuiDataList, TuiDropdown, TuiIcon, TuiInput, TuiNotificationDirective, TuiTitle, TrainerNoteLengthComponent, OptionGroupComponent, TranslocoPipe],
  templateUrl: './trainer-task-pipeline.component.html',
  styleUrl: './trainer-task-pipeline.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerTaskPipelineComponent {

  readonly convert = input.required<TrainerConvert>();
  /** From 1, as the convert is shown. */
  readonly number = input.required<number>();

  readonly trainer = inject(TrainerStore);
  readonly reader = inject(OsuReaderStore);
  private readonly config = inject(ConfigurationStore);
  private readonly transloco = inject(TranslocoService);

  /** The rate change sets the pitch itself (see the settings): no option to show. */
  readonly autoPitch = computed(() => this.config.config()?.trainer.auto_pitch ?? false);

  taskLabels = TASK_LABELS;
  availableTasks: TrainerTaskType[] = ['RateChange', 'SVNormalizer', 'FlnConverter', 'KeyConverter'];
  readonly menuOpen = signal(false);

  availableTasksFiltered = computed(() => {
    const existingTypes = new Set(this.convert().tasks.map(t => t.task.type));
    const mode = this.reader.originalMode();
    return this.availableTasks.filter(type => !existingTypes.has(type) && taskSupportsMode(type, mode));
  });

  /** Whether a task of `type` doesn't work on the map's mode: it stays, but is not applied. */
  isSkipped(type: TrainerTaskType): boolean {
    return !taskSupportsMode(type, this.reader.originalMode());
  }

  /** Why, e.g. `Only for osu!mania maps: skipped on this one`. */
  skippedReason(type: TrainerTaskType): string {
    const modes = TASK_MODES[type];
    const names: Record<string, string> = { osu: 'osu!standard', taiko: 'osu!taiko', fruits: 'osu!catch', mania: 'osu!mania' };
    if (modes === 'all') return this.transloco.translate('trainer.skippedHere');
    const and = this.transloco.translate('trainer.and');
    return this.transloco.translate('trainer.onlyFor', { modes: modes.map(m => names[m]).join(and) });
  }

  originalBpm = computed(() => {
    const bpm = this.reader.data()?.attributes?.bpm.common;
    return bpm ? Math.round(bpm * 100) / 100 : null;
  });

  /** Key count of the current map, only for osu!mania maps. */
  sourceKeys = computed(() => this.reader.data()?.attributes?.keyCount ?? null);

  /** One stage up to 10K; 12K to 18K are two stages of the same size. */
  readonly keyOptions = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 16, 18];
  /** The map's own key count can't be picked. */
  readonly keyItems = computed<OptionGroupItem[]>(() =>
    this.keyOptions.map(keys => ({ label: String(keys), disabled: keys === this.sourceKeys() })),
  );
  /** Labels and hints are translation keys. */
  readonly densityOptions: (DensityOption & OptionGroupItem)[] = (['none', 'dense', 'moreDense', 'full'] as const).map(value => ({
    value,
    label: `trainer.keys.density.${value}`,
    hint: `trainer.keys.density.${value}Hint`,
    translate: true,
  }));

  newBpm(rate: number): number | null {
    const bpm = this.originalBpm();
    return bpm !== null ? Math.round(bpm * rate) : null;
  }

  onDrop(event: CdkDragDrop<unknown>) {
    this.trainer.reorderTasks(this.convert().id, event.previousIndex, event.currentIndex);
  }

  addTask(type: TrainerTaskType) {
    this.menuOpen.set(false);
    switch (type) {
      case 'RateChange':
        this.trainer.addTask(this.convert().id, rateChangeTask(1.0));
        break;
      case 'SVNormalizer':
        this.trainer.addTask(this.convert().id, { type: 'SVNormalizer', params: {} });
        break;
      case 'FlnConverter':
        this.trainer.addTask(this.convert().id, {
          type: 'FlnConverter',
          params: { gap: { unit: 'beat', value: 4 }, minLength: { unit: 'beat', value: 4 }, reworkExistingLn: true },
        });
        break;
      case 'KeyConverter': {
        const source = this.sourceKeys() ?? 4;
        const targetKeys = source < 7 ? 7 : source < 10 ? 10 : 7;
        this.trainer.addTask(this.convert().id, {
          type: 'KeyConverter',
          params: { targetKeys, density: 'none', keepJacks: true, playstyle: null },
        });
        break;
      }
    }
  }

  onRateInput(id: string, rateStr: string, params: RateChangeParams) {
    const rate = parseFloat(rateStr);
    if (isNaN(rate) || rate <= 0) return;
    this.trainer.updateTaskParams(this.convert().id, id, { ...params, rate });
  }

  onBpmInput(id: string, bpmStr: string, params: RateChangeParams) {
    const bpm = this.originalBpm();
    if (bpm === null) return;
    const newBpm = parseFloat(bpmStr);
    if (isNaN(newBpm) || newBpm <= 0) return;
    const rate = Math.round((newBpm / bpm) * 100) / 100;
    this.trainer.updateTaskParams(this.convert().id, id, { ...params, rate });
  }

  onRateOptionChange(id: string, params: RateChangeParams, change: Partial<RateChangeParams>) {
    this.trainer.updateTaskParams(this.convert().id, id, { ...params, ...change });
  }

  onKeyChange(id: string, params: KeyConverterParams, change: Partial<KeyConverterParams>) {
    this.trainer.updateTaskParams(this.convert().id, id, { ...params, ...change });
  }

  /** Changing the key count resets the playstyle, and the extra notes that no longer apply. */
  onTargetKeys(id: string, params: KeyConverterParams, targetKeys: number) {
    const density = this.isReducing(targetKeys) ? 'none' : params.density;
    this.onKeyChange(id, params, { targetKeys, density, playstyle: null });
  }

  isReducing(targetKeys: number): boolean {
    const source = this.sourceKeys();
    return source !== null && targetKeys < source;
  }

  densityIndex(density: KeyDensity): number {
    return Math.max(0, this.densityOptions.findIndex(o => o.value === density));
  }

  playstyles(targetKeys: number): { name: string; hint: string }[] {
    return PLAYSTYLES[targetKeys] ?? [];
  }

  /** The playstyles of `targetKeys` as options, the same list each time. */
  playstyleItems(targetKeys: number): OptionGroupItem[] {
    return (this.playstyleOptions[targetKeys] ??= this.playstyles(targetKeys).map(p => ({ label: p.name })));
  }
  private readonly playstyleOptions: Record<number, OptionGroupItem[]> = {};

  playstyleIndex(params: KeyConverterParams): number {
    return Math.max(0, this.playstyles(params.targetKeys).findIndex(p => p.name === params.playstyle));
  }

  /** The usual playstyle is sent as `null`, so the diff name only mentions the others. */
  onPlaystyle(id: string, params: KeyConverterParams, index: number) {
    const playstyle = index === 0 ? null : this.playstyles(params.targetKeys)[index].name;
    this.onKeyChange(id, params, { playstyle });
  }

  onFlnChange(id: string, params: FlnConverterParams, change: Partial<FlnConverterParams>) {
    this.trainer.updateTaskParams(this.convert().id, id, { ...params, ...change });
  }
}
