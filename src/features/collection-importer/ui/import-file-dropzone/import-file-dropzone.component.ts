import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { NotifyService } from '@shared/services';
import { ButtonComponent, IconComponent, OptionGroupComponent, OptionGroupItem, TxtDropzoneComponent } from '@shared/ui';
import { ImportCollectionParserService } from '../../import-collection-parser.service';
import { ImportCollectionStore } from '../../import-collections.store';

/**
 * Step 1: the collections to import, from a .txt file or pasted in, as the
 * exporter writes them. Once read, what it brought, and a way to drop it.
 */
@Component({
  selector: 'app-import-file-dropzone',
  imports: [ButtonComponent, IconComponent, OptionGroupComponent, TranslocoPipe, TxtDropzoneComponent],
  templateUrl: './import-file-dropzone.component.html',
  styleUrl: './import-file-dropzone.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportFileDropzoneComponent {
  readonly store = inject(ImportCollectionStore);
  private readonly parser = inject(ImportCollectionParserService);
  private readonly notify = inject(NotifyService);
  private readonly transloco = inject(TranslocoService);

  readonly modes: OptionGroupItem[] = [
    { label: 'collections.import.fromFile', icon: 'file-text', translate: true },
    { label: 'collections.import.fromPaste', icon: 'clipboard-paste', translate: true },
  ];
  readonly mode = signal(0);

  /** What is pasted, and the collections found in it as it is typed. */
  readonly text = signal('');
  readonly found = computed(() => this.parser.parse(this.text()).length);

  load(content: string) {
    const parsed = this.parser.parse(content);
    if (parsed.length === 0) {
      this.notify.error(this.transloco.translate('collections.import.nothing'));
      return;
    }
    this.store.setParsed(parsed);
    this.text.set('');
  }
}
