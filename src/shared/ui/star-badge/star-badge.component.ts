import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { starBg, starFg } from '@shared/utils';
import { TuiBadge } from '@taiga-ui/kit';

/** A star rating in osu!'s colours, e.g. `<app-star-badge size="s" [stars]="diff.stars" />`. */
@Component({
  selector: 'app-star-badge',
  imports: [DecimalPipe],
  template: `★ {{ stars() | number: '1.2-2' }}`,
  // The element itself is the badge, so it lays out and is styled as one.
  hostDirectives: [{ directive: TuiBadge, inputs: ['size'] }],
  host: {
    // What Taiga's badge styles select.
    tuiBadge: '',
    '[style.background]': 'background()',
    '[style.color]': 'color()',
  },
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class StarBadgeComponent {
  readonly stars = input.required<number>();

  readonly background = computed(() => starBg(this.stars()));
  readonly color = computed(() => starFg(this.stars()));
}
