import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import type { LogEntry } from '@entities/log';
import { TuiItem } from '@taiga-ui/cdk';
import { TuiExpand, TuiScrollbar } from '@taiga-ui/core';
import { TuiAccordion, TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

const LEVEL_APPEARANCE: Record<string, string> = {
  DEBUG: 'info',
  INFO: 'primary',
  WARN: 'warning',
  ERROR: 'negative',
};

@Component({
  selector: 'app-log-column',
  imports: [TuiAccordion, TuiBadge, TuiCardLarge, TuiExpand, TuiItem, TuiScrollbar],
  templateUrl: './log-column.component.html',
  styleUrl: './log-column.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LogColumnComponent {
  readonly level = input.required<string>();
  readonly groups = input.required<Record<string, LogEntry[]>>();

  readonly appearance = computed(() => LEVEL_APPEARANCE[this.level()] ?? 'neutral');

  readonly targetNames = computed(() => Object.keys(this.groups()));

  readonly totalCount = computed(() =>
    Object.values(this.groups()).reduce((sum, entries) => sum + entries.length, 0)
  );

  entriesFor(target: string): LogEntry[] {
    return this.groups()[target] ?? [];
  }
}
