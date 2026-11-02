import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { TuiNotificationDirective, TuiTitle } from '@taiga-ui/core';

/** Warns that `count` sets exported as links were never checked online by osu! (see `isUncheckedSet`). */
@Component({
  selector: 'app-unchecked-notice',
  imports: [TuiNotificationDirective, TuiTitle],
  template: `
    <div appearance="warning" size="s" tuiNotification>
      <div tuiTitle>
        {{ count() }} {{ count() === 1 ? 'beatmapset' : 'beatmapsets' }} not checked by osu! yet
        <span tuiSubtitle>
          osu! finds out whether a map is on the website when it checks it online, in song select.
          These have an id, so they are exported as uploaded, but some may not be.
        </span>
      </div>
    </div>
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UncheckedNoticeComponent {
  readonly count = input.required<number>();
}
