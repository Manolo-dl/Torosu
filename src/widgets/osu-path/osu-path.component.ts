import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { OsuPathStore } from '@entities/osu-path';
import { TuiButton, TuiNotificationDirective, TuiTitle } from "@taiga-ui/core";
import { TuiButtonLoading } from '@taiga-ui/kit';

/** Asks for the osu! folder when none was found. Once set, it is changed in the settings. */
@Component({
  selector: 'app-osu-path',
  imports: [TuiButton, TuiButtonLoading, TuiNotificationDirective, TuiTitle],
  templateUrl: './osu-path.component.html',
  styleUrl: './osu-path.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OsuPathComponent {
  readonly osuPath = inject(OsuPathStore);
}
