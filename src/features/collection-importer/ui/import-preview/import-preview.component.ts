import { BadgeComponent, CardComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { ImportCollectionStore } from '../../import-collections.store';
import { BeatmapSetCardComponent } from '@entities/osu-db/ui';
import { TuiExpand, TuiScrollbar, TuiTitle } from '@taiga-ui/core';
import { TuiAccordion} from '@taiga-ui/kit';

@Component({
  selector: 'app-import-preview',
  imports: [BeatmapSetCardComponent, TuiAccordion, TuiExpand, TuiScrollbar, TuiTitle, TranslocoPipe, BadgeComponent, CardComponent],
  templateUrl: './import-preview.component.html',
  styleUrl: './import-preview.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportPreviewComponent {
  readonly store = inject(ImportCollectionStore);
}
