import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ImportCollectionParserService } from '@features/collection-importer/import-collection-parser.service';
import { ImportCollectionStore } from '@features/collection-importer/import-collections.store';
import { TxtFileDropDirective } from '@shared/directives';
import { TuiIcon, TuiLink, TuiTitle } from '@taiga-ui/core';
import { TuiFiles } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-import-file-dropzone',
  imports: [FormsModule, TxtFileDropDirective, TuiCardLarge, TuiFiles, TuiIcon, TuiLink, TuiTitle],
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

  onFileSelect(file: File | null) {
    if (file) this.readFile(file);
  }

  private readFile(file: File) {
    const reader = new FileReader();
    reader.onload = (e) => {
      this.onFileContent(e.target?.result as string);
    };

    reader.readAsText(file);
  }
}
