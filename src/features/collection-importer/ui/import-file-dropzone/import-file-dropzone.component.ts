import { Component, inject, OnDestroy, signal } from '@angular/core';
import { ImportCollectionParserService } from '@features/collection-importer/import-collection-parser.service';
import { ImportCollectionStore } from '@features/collection-importer/import-collections.store';
import { TxtFileDropDirective } from '@shared/directives';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { readTextFile } from '@tauri-apps/plugin-fs';

@Component({
  selector: 'app-import-file-dropzone',
  imports: [TxtFileDropDirective],
  templateUrl: './import-file-dropzone.component.html',
  styleUrl: './import-file-dropzone.component.css',
})
export class ImportFileDropzoneComponent {
  private store = inject(ImportCollectionStore);
  private parser = inject(ImportCollectionParserService);
  
  onFileContent(content: string) {
    const parsed = this.parser.parse(content);
    this.store.setParsed(parsed);
  }

  onFileSelect(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (file) this.readFile(file);
    input.value = '';
  }

  private readFile(file: File) {
    const reader = new FileReader();
    reader.onload = (e) => {
      this.onFileContent(e.target?.result as string);
    };

    reader.readAsText(file);
  }
}
