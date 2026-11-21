import { IconComponent } from '../icon/icon.component';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, output } from '@angular/core';
import { TxtFileDropDirective } from '@shared/directives';

/**
 * Where a .txt file is dropped (anywhere on the window) or picked. Its
 * height is `--txt-dropzone-height`, 8rem unless set.
 */
@Component({
  selector: 'app-txt-dropzone',
  imports: [TxtFileDropDirective, TranslocoPipe, IconComponent],
  templateUrl: './txt-dropzone.component.html',
  styleUrl: './txt-dropzone.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TxtDropzoneComponent {
  /** The text of each file. */
  readonly fileContent = output<string>();

  async onPick(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    // Emptied, so the same file can be picked again.
    input.value = '';
    if (file) this.fileContent.emit(await file.text());
  }
}
