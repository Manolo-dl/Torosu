import { BadgeComponent, CardComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { ScrollingModule } from '@angular/cdk/scrolling';
import { SelectedDiff } from '../../pack.model';
import { PackStore } from '../../pack.store';
import { TuiScrollControls, TuiScrollRef, TuiTitle } from '@taiga-ui/core';
import { PackPreviewCardComponent } from "../pack-preview-card/pack-preview-card.component";

@Component({
  selector: 'app-pack-preview',
  imports: [ScrollingModule, PackPreviewCardComponent, TuiScrollControls, TuiScrollRef, TuiTitle, TranslocoPipe, BadgeComponent, CardComponent],
  templateUrl: './pack-preview.component.html',
  styleUrl: './pack-preview.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PackPreviewComponent {

  readonly store = inject(PackStore);

  trackDiff(_index: number, diff: SelectedDiff): string {
    return diff.md5;
  }
}
