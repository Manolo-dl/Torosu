import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { BeatmapExporterStore, ModeFilters, emptyModeFilters } from '@features/export-songs/stores/beatmap-exporter.store';

export interface ModeConfig {
  label: string;
  value: number;
  icon: string;
  color: string;
  hasAr: boolean;
  csLabel: string | null;
}

@Component({
  selector: 'app-beatmap-filters',
  imports: [],
  templateUrl: './beatmap-filters.component.html',
  styleUrl: './beatmap-filters.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapFiltersComponent {
  readonly store = inject(BeatmapExporterStore);

  readonly modeConfigs: ModeConfig[] = [
    { label: 'Standard', value: 0, icon: '●', color: '#ff66ab', hasAr: true,  csLabel: 'CS'   },
    { label: 'Taiko',    value: 1, icon: '◉', color: '#e05555', hasAr: false, csLabel: null    },
    { label: 'Catch',    value: 2, icon: '✿', color: '#66bbff', hasAr: true,  csLabel: 'CS'   },
    { label: 'Mania',    value: 3, icon: '⊞', color: '#c966ff', hasAr: false, csLabel: 'Keys' },
  ];

  readonly statuses = ['Ranked', 'Loved', 'Approved', 'Qualified', 'Unranked'];
  readonly keyCounts = [4, 5, 6, 7, 8];

  private expandedModes = signal<number[]>([]);

  // ── Expand / collapse ───────────────────────────────────────────────────────

  isExpanded(mode: number): boolean {
    return this.expandedModes().includes(mode);
  }

  toggleExpand(mode: number) {
    this.expandedModes.update(cur =>
      cur.includes(mode) ? cur.filter(m => m !== mode) : [...cur, mode]
    );
  }

  // ── Include / exclude mode ──────────────────────────────────────────────────

  isModeExcluded(mode: number): boolean {
    return this.store.filters().excludedModes.includes(mode);
  }

  toggleExclude(mode: number) {
    const current = this.store.filters().excludedModes;
    const wasExcluded = current.includes(mode);
    this.store.setFilters({
      excludedModes: wasExcluded ? current.filter(m => m !== mode) : [...current, mode],
    });
    // Auto-expand when enabling a mode
    if (wasExcluded) {
      this.expandedModes.update(e => e.includes(mode) ? e : [...e, mode]);
    }
  }

  // ── Per-mode filters ────────────────────────────────────────────────────────

  modeFilters(mode: number): ModeFilters {
    return this.store.filters().modeFilters[mode] ?? emptyModeFilters();
  }

  setModeFilter(mode: number, key: keyof ModeFilters, value: ModeFilters[typeof key]) {
    this.store.setModeFilters(mode, { [key]: value });
  }

  setModeMin(mode: number, minKey: keyof ModeFilters, maxKey: keyof ModeFilters, value: string) {
    const min = value ? parseFloat(value) : null;
    const currentMax = this.modeFilters(mode)[maxKey] as number | null;
    this.store.setModeFilters(mode, {
      [minKey]: min,
      ...(min !== null && (currentMax === null || min > currentMax) ? { [maxKey]: min } : {}),
    });
  }

  setModeMax(mode: number, minKey: keyof ModeFilters, maxKey: keyof ModeFilters, value: string) {
    const max = value ? parseFloat(value) : null;
    const currentMin = this.modeFilters(mode)[minKey] as number | null;
    this.store.setModeFilters(mode, {
      [maxKey]: max,
      ...(max !== null && currentMin !== null && max < currentMin ? { [minKey]: max } : {}),
    });
  }

  setModeMinLength(mode: number, value: string) {
    const min = value ? parseInt(value) * 1000 : null;
    const currentMax = this.modeFilters(mode).maxLength;
    this.store.setModeFilters(mode, {
      minLength: min,
      ...(min !== null && (currentMax === null || min > currentMax) ? { maxLength: min } : {}),
    });
  }

  setModeMaxLength(mode: number, value: string) {
    const max = value ? parseInt(value) * 1000 : null;
    const currentMin = this.modeFilters(mode).minLength;
    this.store.setModeFilters(mode, {
      maxLength: max,
      ...(max !== null && currentMin !== null && max < currentMin ? { minLength: max } : {}),
    });
  }

  setModeKeyCount(mode: number, value: number | null) {
    this.store.setModeFilters(mode, { minCs: value, maxCs: value });
  }

  modeKeyCount(mode: number): number | null {
    const mf = this.modeFilters(mode);
    return mf.minCs !== null && mf.minCs === mf.maxCs ? mf.minCs : null;
  }

  msToSec(ms: number | null): number | '' {
    return ms !== null ? ms / 1000 : '';
  }

  // ── Global ──────────────────────────────────────────────────────────────────

  setLastPlayed(value: boolean | null) {
    this.store.setFilters({ lastPlayed: value });
  }

  reset() {
    this.store.resetFilters();
    this.expandedModes.set([]);
  }
}
