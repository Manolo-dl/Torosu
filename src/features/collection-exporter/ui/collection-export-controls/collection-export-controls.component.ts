import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { UncheckedNoticeComponent } from '@entities/osu-db/ui';
import { IconComponent } from '@shared/ui';
import { CollectionFormat, ExportCollectionsStore } from '../../export-collections.store';

interface FormatOption {
  value: CollectionFormat;
  icon: string;
  /** Translation keys. */
  title: string;
  hint: string;
}

const FORMATS: FormatOption[] = [
  { value: 'links', icon: 'link', title: 'collections.export.links', hint: 'collections.export.linksHint' },
  { value: 'md5', icon: 'hash', title: 'collections.export.md5', hint: 'collections.export.md5Hint' },
];

/**
 * Step 2: how the collections are written, each way with what it is for and
 * what it carries of the ones picked.
 */
@Component({
  selector: 'app-collection-export-controls',
  imports: [IconComponent, TranslocoPipe, UncheckedNoticeComponent],
  templateUrl: './collection-export-controls.component.html',
  styleUrl: './collection-export-controls.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionExportControlsComponent {
  readonly store = inject(ExportCollectionsStore);
  readonly formats = FORMATS;
}
