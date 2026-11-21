import { BadgeComponent, CardComponent, BadgeTone } from '@shared/ui';
import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import type { LogEntry } from '@entities/log';
import { TuiItem } from '@taiga-ui/cdk';
import { TuiExpand, TuiScrollbar } from '@taiga-ui/core';
import { TuiAccordion} from '@taiga-ui/kit';

const LEVEL_TONE: Record<string, BadgeTone> = {
  DEBUG: 'info',
  INFO: 'accent',
  WARN: 'warning',
  ERROR: 'danger',
};

@Component({
  selector: 'app-log-column',
  imports: [TuiAccordion, TuiExpand, TuiItem, TuiScrollbar, BadgeComponent, CardComponent],
  templateUrl: './log-column.component.html',
  styleUrl: './log-column.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LogColumnComponent {
  readonly level = input.required<string>();
  readonly groups = input.required<Record<string, LogEntry[]>>();

  readonly tone = computed<BadgeTone>(() => LEVEL_TONE[this.level()] ?? 'neutral');

  readonly targetNames = computed(() => Object.keys(this.groups()));

  readonly totalCount = computed(() =>
    Object.values(this.groups()).reduce((sum, entries) => sum + entries.length, 0)
  );

  entriesFor(target: string): LogEntry[] {
    return this.groups()[target] ?? [];
  }
}
