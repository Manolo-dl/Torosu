import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { ImportCollectionStore } from '../../import-collections.store';
import { BeatmapSetCardComponent } from '@entities/osu-db/ui';
import { TuiExpand, TuiScrollbar, TuiTitle } from '@taiga-ui/core';
import { TuiAccordion, TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-import-preview',
  imports: [BeatmapSetCardComponent, TuiAccordion, TuiBadge, TuiCardLarge, TuiExpand, TuiScrollbar, TuiTitle],
  templateUrl: './import-preview.component.html',
  styleUrl: './import-preview.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportPreviewComponent {
  readonly store = inject(ImportCollectionStore);
}
