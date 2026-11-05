import { DecimalPipe } from '@angular/common';
import { booleanAttribute, ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { TuiBadge } from '@taiga-ui/kit';
import { Dan, Skill } from '../../osu-reader.model';

/** What to show of a map's skill; the trainer's `skill_display` setting. */
export type SkillDisplay = 'dan' | 'msd' | 'both' | 'none';

/**
 * A map's dan and/or MSD as badges, e.g.
 * `<app-skill-badges size="l" [skill]="reader.skill()" display="both" [keyCount]="4" />`.
 * Nothing for what the map doesn't have (a 5K map has no dan).
 */
@Component({
  selector: 'app-skill-badges',
  imports: [DecimalPipe, TuiBadge, TranslocoPipe],
  template: `
    @if (showDan()) {
      @for (dan of dans(); track dan.family) {
        <span
          tuiBadge
          class="dan"
          [class.ln]="dan.family === 'ln'"
          [size]="size()"
          [attr.title]="'trainer.dan.hint' | transloco: { confidence: (dan.confidence * 100 | number: '1.0-0') }"
        >
          {{ 'trainer.dan.' + dan.family | transloco: { name: dan.name } }}
        </span>
      }
    }
    @if (showMsd() && skill()?.msd; as msd) {
      <span tuiBadge class="msd" [size]="size()" [attr.title]="msdHint()">
        {{ 'trainer.msd.badge' | transloco: { value: (msd.overall | number: '1.2-2') } }}
      </span>
    }
  `,
  styleUrl: './skill-badges.component.less',
  host: { '[class.on-image]': 'onImage()' },
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SkillBadgesComponent {
  readonly skill = input.required<Skill | null>();
  readonly display = input.required<SkillDisplay>();
  /** Of the map rated: tells MinaCalc's MSD from Prism's. */
  readonly keyCount = input<number | null>(null);
  readonly size = input<'s' | 'm' | 'l'>('m');
  /** Over a beatmap background: fixed colours, in both themes. */
  readonly onImage = input(false, { transform: booleanAttribute });

  private readonly transloco = inject(TranslocoService);

  readonly showDan = computed(() => this.display() === 'dan' || this.display() === 'both');
  readonly showMsd = computed(() => this.display() === 'msd' || this.display() === 'both');

  /** The primary dan first, then the other one of a hybrid map. */
  readonly dans = computed((): Dan[] => {
    const dan = this.skill()?.dan;
    if (!dan) return [];
    const other = dan.primary.family === 'rice' ? dan.ln : dan.rice;
    return other ? [dan.primary, other] : [dan.primary];
  });

  /** `Prism MSD` and a skillset a line. */
  readonly msdHint = computed(() => {
    const msd = this.skill()?.msd;
    if (!msd) return '';
    const keys = this.keyCount();
    const calc = keys !== null && keys >= 6 && keys <= 8 ? 'Prism' : 'MinaCalc';
    const lines = msd.skillsets.map(s => `${s.name}: ${s.value.toFixed(2)}`);
    return [this.transloco.translate('trainer.msd.hint', { calc }), ...lines].join('\n');
  });
}
