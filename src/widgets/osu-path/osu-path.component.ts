import { ButtonComponent, NoticeComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { OsuPathStore } from '@entities/osu-path';

/** Asks for the osu! folder when none was found. Once set, it is changed in the settings. */
@Component({
  selector: 'app-osu-path',
  imports: [TranslocoPipe, ButtonComponent, NoticeComponent],
  templateUrl: './osu-path.component.html',
  styleUrl: './osu-path.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OsuPathComponent {
  readonly osuPath = inject(OsuPathStore);
}
