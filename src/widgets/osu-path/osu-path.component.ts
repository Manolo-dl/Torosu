import { ChangeDetectionStrategy, Component, inject, OnInit } from '@angular/core';
import { OsuPathStore } from '@shared/stores';
import { TuiSkeleton } from '@taiga-ui/kit';
import { TuiButton, TuiTitle } from "@taiga-ui/core";

@Component({
  selector: 'app-osu-path',
  imports: [TuiSkeleton, TuiButton, TuiTitle],
  templateUrl: './osu-path.component.html',
  styleUrl: './osu-path.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OsuPathComponent implements OnInit {

  osuPath = inject(OsuPathStore);

  ngOnInit(): void {
    this.osuPath.detect();
  }
}
