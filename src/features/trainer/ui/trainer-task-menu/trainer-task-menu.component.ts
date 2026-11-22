import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { TuiDataList } from '@taiga-ui/core';
import { IconComponent } from '@shared/ui';
import { TASK_LABELS, TASK_LOOKS, TrainerTaskType } from '../../models/trainer.model';

/** The tasks that can be added to a convert, each with what it does. */
@Component({
  selector: 'app-trainer-task-menu',
  imports: [TuiDataList, IconComponent, TranslocoPipe],
  template: `
    <tui-data-list>
      @for (type of types(); track type) {
        <button tuiOption type="button" [style.--task-hue]="looks[type].hue" (click)="pick.emit(type)">
          <span class="tile"><app-icon [name]="looks[type].icon" /></span>
          <span class="text">
            <strong>{{ labels[type] | transloco }}</strong>
            <small>{{ 'trainer.taskHints.' + type | transloco }}</small>
          </span>
        </button>
      }
    </tui-data-list>
  `,
  styleUrl: './trainer-task-menu.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerTaskMenuComponent {
  readonly types = input.required<readonly TrainerTaskType[]>();
  readonly pick = output<TrainerTaskType>();

  readonly labels = TASK_LABELS;
  readonly looks = TASK_LOOKS;
}
