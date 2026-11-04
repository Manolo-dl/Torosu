import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { TuiNotificationDirective, TuiTitle } from '@taiga-ui/core';

/** Warns that `count` sets exported as links were never checked online by osu! (see `isUncheckedSet`). */
@Component({
  selector: 'app-unchecked-notice',
  imports: [TuiNotificationDirective, TuiTitle, TranslocoPipe],
  template: `
    <div appearance="warning" size="s" tuiNotification>
      <div tuiTitle>
        {{ count() === 1 ? ('unchecked.titleOne' | transloco) : ('unchecked.title' | transloco: { count: count() }) }}
        <span tuiSubtitle>{{ 'unchecked.hint' | transloco }}</span>
      </div>
    </div>
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UncheckedNoticeComponent {
  readonly count = input.required<number>();
}
