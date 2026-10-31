import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { ImportCollectionParserService } from '../../import-collection-parser.service';
import { ImportCollectionStore } from '../../import-collections.store';
import { TxtDropzoneComponent } from '@shared/ui';
import { TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-import-file-dropzone',
  imports: [TxtDropzoneComponent, TuiCardLarge, TuiTitle],
  templateUrl: './import-file-dropzone.component.html',
  styleUrl: './import-file-dropzone.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportFileDropzoneComponent {
  private store = inject(ImportCollectionStore);
  private parser = inject(ImportCollectionParserService);

  onFileContent(content: string) {
    const parsed = this.parser.parse(content);
    this.store.setParsed(parsed);
  }
}
