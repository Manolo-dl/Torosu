import { DecimalPipe } from '@angular/common';
import { booleanAttribute, ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { msdColor, textOn } from '@shared/utils';
import { Dan, Skill } from '../../osu-reader.model';
import { DanEmblemComponent } from '../dan-emblem/dan-emblem.component';

/** What to show of a map's skill; the trainer's `skill_display` setting. */
export type SkillDisplay = 'dan' | 'msd' | 'both' | 'none';

/** A change from the map compared with, e.g. `▲ 1.2`; `null` when there is none to tell. */
interface Delta {
  text: string;
  up: boolean;
}

/**
 * A map's dan and/or MSD: each dan as its emblem and name, the MSD as its
 * value in its colour with the skillset it comes from. Nothing for what the
 * map doesn't have (a 5K map has no dan).
 *
 * `<app-skill-summary size="l" [skill]="reader.skill()" display="both" />`
 */
@Component({
  selector: 'app-skill-summary',
  imports: [DanEmblemComponent, DecimalPipe, TranslocoPipe],
  templateUrl: './skill-summary.component.html',
  styleUrl: './skill-summary.component.less',
  host: {
    '[class.on-image]': 'onImage()',
    '[class.size-l]': "size() === 'l'",
  },
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SkillSummaryComponent {
  readonly skill = input.required<Skill | null>();
  readonly display = input.required<SkillDisplay>();
  readonly size = input<'m' | 'l'>('m');
  /** Over a beatmap background: light text on dark, in both themes. */
  readonly onImage = input(false, { transform: booleanAttribute });
  /** Another map's skill, to show how much harder or easier this one is (a convert against its map). */
  readonly compareTo = input<Skill | null>(null);

  private readonly transloco = inject(TranslocoService);

  readonly showDan = computed(() => this.display() === 'dan' || this.display() === 'both');
  readonly showMsd = computed(() => this.display() === 'msd' || this.display() === 'both');
  readonly emblemSize = computed(() => (this.size() === 'l' ? 46 : 38));
  /** Of the map rated, not the one shown now: picks the badges. */
  readonly keyCount = computed(() => this.skill()?.keyCount ?? null);

  /** The primary dan first, then the other one of a hybrid map. */
  readonly dans = computed((): Dan[] => {
    const dan = this.skill()?.dan;
    if (!dan) return [];
    const other = dan.primary.family === 'rice' ? dan.ln : dan.rice;
    return other ? [dan.primary, other] : [dan.primary];
  });

  readonly msd = computed(() => this.skill()?.msd ?? null);
  readonly msdBg = computed(() => msdColor(this.msd()?.overall ?? 0));
  readonly msdFg = computed(() => textOn(this.msdBg()));

  /** The skillset the overall comes from: the highest. */
  readonly topSkillset = computed(() => {
    const skillsets = this.msd()?.skillsets ?? [];
    return skillsets.reduce<{ name: string; value: number } | null>((top, s) => (!top || s.value > top.value ? s : top), null)?.name ?? '';
  });

  /** `MinaCalc` or `Prism`. */
  readonly calculator = computed(() => (this.msd()?.calculator === 'prism' ? 'Prism' : 'MinaCalc'));

  readonly msdHint = computed(() => {
    const msd = this.msd();
    if (!msd) return '';
    const lines = msd.skillsets.map(s => `${s.name}: ${s.value.toFixed(2)}`);
    return [this.transloco.translate('trainer.msd.hint', { calc: this.calculator() }), ...lines].join('\n');
  });

  /** Only against an MSD on the same scale (see `sameMsdScale`). */
  readonly msdDelta = computed(() => {
    const [skill, other] = [this.skill(), this.compareTo()];
    if (!skill?.msd || !other?.msd || !sameMsdScale(skill, other)) return null;
    return delta(skill.msd.overall - other.msd.overall, 2);
  });

  danHint(dan: Dan): string {
    return this.transloco.translate('trainer.dan.hint', { confidence: Math.round(dan.confidence * 100) });
  }

  /**
   * Against the compared map's dan of the same family (rice with rice, LN with
   * LN) and keymode: 4K, 6K and 7K dans are different courses, not one scale.
   */
  danDelta(dan: Dan): Delta | null {
    const other = this.compareTo();
    if (!other || other.keyCount !== this.keyCount()) return null;
    const before = dan.family === 'rice' ? other.dan?.rice : other.dan?.ln;
    return before ? delta(dan.value - before.value, 1) : null;
  }

  deltaHint(delta: Delta): string {
    return this.transloco.translate('trainer.skill.vsMap', { delta: delta.text });
  }
}

/**
 * Whether the MSDs of `a` and `b` are on the same scale: from the same
 * calculator, and for MinaCalc from the same build, as dan-calc picks them
 * (0.72.3 on 4K, 0.75.0 on the other keymodes Prism doesn't rate).
 */
function sameMsdScale(a: Skill, b: Skill): boolean {
  if (a.msd?.calculator !== b.msd?.calculator) return false;
  return a.msd?.calculator === 'prism' || (a.keyCount === 4) === (b.keyCount === 4);
}

function delta(change: number, digits: number): Delta | null {
  const rounded = Number(change.toFixed(digits));
  if (rounded === 0) return null;
  return { text: `${rounded > 0 ? '+' : '−'}${Math.abs(rounded).toFixed(digits)}`, up: rounded > 0 };
}
