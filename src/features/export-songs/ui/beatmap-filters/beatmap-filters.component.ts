import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { BeatmapExporterStore, ModeFilters, emptyModeFilters } from '../../stores/beatmap-exporter.store';
import { TuiButton, TuiExpand, TuiIcon, TuiInput, TuiTitle } from '@taiga-ui/core';
import { TuiChip, TuiSegmented, TuiSwitch } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';
import { GAME_MODES } from '@shared/utils';

export interface ModeConfig {
  label: string;
  value: number;
  icon: string;
  color: string;
  hasAr: boolean;
  csLabel: string | null;
}

/** How a mode looks, for its filters. */
function mode(value: number): Pick<ModeConfig, 'label' | 'value' | 'icon' | 'color'> {
  const { shortName, icon, color } = GAME_MODES[value];
  return { label: shortName, value, icon, color };
}

type RangeKey = { [K in keyof ModeFilters]: ModeFilters[K] extends number | null ? K : never }[keyof ModeFilters];

interface RangeField {
  label: string;
  min: RangeKey;
  max: RangeKey;
  /** The highest value that can be typed; none if not given. */
  limit?: number;
  /** 0.5 if not given. */
  step?: number;
  /** What a typed value is multiplied by to be stored: 1000 for seconds kept in ms. */
  scale?: number;
}

@Component({
  selector: 'app-beatmap-filters',
  imports: [FormsModule, TuiButton, TuiCardLarge, TuiChip, TuiExpand, TuiIcon, TuiInput, TuiSegmented, TuiSwitch, TuiTitle],
  templateUrl: './beatmap-filters.component.html',
  styleUrl: './beatmap-filters.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapFiltersComponent {
  readonly store = inject(BeatmapExporterStore);

  readonly modeConfigs: ModeConfig[] = [
    { ...mode(0), hasAr: true,  csLabel: 'CS'   },
    { ...mode(1), hasAr: false, csLabel: null   },
    { ...mode(2), hasAr: true,  csLabel: 'CS'   },
    { ...mode(3), hasAr: false, csLabel: 'Keys' },
  ];

  readonly statuses = ['Ranked', 'Loved', 'Approved', 'Qualified', 'Unranked'];
  readonly keyCounts = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 16, 18];
  readonly playedOptions: (boolean | null)[] = [null, true, false];

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

  readonly rangeFields = new Map(this.modeConfigs.map(m => [m.value, this.buildRangeFields(m)]));

  private buildRangeFields(mode: ModeConfig): RangeField[] {
    return [
      { label: 'Length (s)', min: 'minLength', max: 'maxLength', step: 1, scale: 1000 },
      { label: 'Stars', min: 'minStars', max: 'maxStars', limit: 15 },
      ...(mode.hasAr ? [{ label: 'AR', min: 'minAr', max: 'maxAr', limit: 10 } as RangeField] : []),
      { label: 'OD', min: 'minOd', max: 'maxOd', limit: 10 },
      { label: 'HP', min: 'minHp', max: 'maxHp', limit: 10 },
      ...(mode.csLabel && mode.value !== 3 ? [{ label: mode.csLabel, min: 'minCs', max: 'maxCs', limit: 10 } as RangeField] : []),
    ];
  }

  // ── Per-mode filters ────────────────────────────────────────────────────────

  modeFilters(mode: number): ModeFilters {
    return this.store.filters().modeFilters[mode] ?? emptyModeFilters();
  }

  setModeFilter(mode: number, key: keyof ModeFilters, value: ModeFilters[typeof key]) {
    this.store.setModeFilters(mode, { [key]: value });
  }

  /** What the input of `key` shows: the stored value, in the unit of `field`. */
  rangeValue(mode: number, field: RangeField, key: RangeKey): number | '' {
    const value = this.modeFilters(mode)[key];
    return value !== null ? value / (field.scale ?? 1) : '';
  }

  setModeMin(mode: number, field: RangeField, value: string) {
    const min = value ? parseFloat(value) * (field.scale ?? 1) : null;
    const currentMax = this.modeFilters(mode)[field.max];
    this.store.setModeFilters(mode, {
      [field.min]: min,
      ...(min !== null && currentMax !== null && min > currentMax ? { [field.max]: min } : {}),
    });
  }

  setModeMax(mode: number, field: RangeField, value: string) {
    const max = value ? parseFloat(value) * (field.scale ?? 1) : null;
    const currentMin = this.modeFilters(mode)[field.min];
    this.store.setModeFilters(mode, {
      [field.max]: max,
      ...(max !== null && currentMin !== null && max < currentMin ? { [field.min]: max } : {}),
    });
  }

  setModeKeyCount(mode: number, value: number | null) {
    this.store.setModeFilters(mode, { minCs: value, maxCs: value });
  }

  modeKeyCount(mode: number): number | null {
    const mf = this.modeFilters(mode);
    return mf.minCs !== null && mf.minCs === mf.maxCs ? mf.minCs : null;
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
