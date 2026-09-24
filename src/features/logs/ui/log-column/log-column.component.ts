import { ChangeDetectionStrategy, Component, computed, input, signal } from '@angular/core';
import type { LogEntry } from '@entities/log';

@Component({
  selector: 'app-log-column',
  imports: [],
  templateUrl: './log-column.component.html',
  styleUrl: './log-column.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LogColumnComponent {
  readonly level = input.required<string>();
  readonly groups = input.required<Record<string, LogEntry[]>>();

  private readonly expanded = signal(new Set<string>());

  readonly targetNames = computed(() => Object.keys(this.groups()));

  readonly totalCount = computed(() =>
    Object.values(this.groups()).reduce((sum, entries) => sum + entries.length, 0)
  );

  isExpanded(target: string): boolean {
    return this.expanded().has(target);
  }

  toggle(target: string) {
    const next = new Set(this.expanded());
    if (next.has(target)) next.delete(target);
    else next.add(target);
    this.expanded.set(next);
  }

  entriesFor(target: string): LogEntry[] {
    return this.groups()[target] ?? [];
  }
}
