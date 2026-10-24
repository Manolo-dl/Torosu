import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { OsuReaderStore } from '@entities/osu-reader';
import { starBg, starFg } from '@shared/utils';
import { TuiTitle } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';

@Component({
  selector: 'app-trainer-beatmap-header',
  imports: [DecimalPipe, TuiBadge, TuiTitle],
  templateUrl: './beatmap-header.component.html',
  styleUrl: './beatmap-header.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerBeatmapHeaderComponent {
  readonly reader = inject(OsuReaderStore);
  readonly starBg = starBg;
  readonly starFg = starFg;
}
