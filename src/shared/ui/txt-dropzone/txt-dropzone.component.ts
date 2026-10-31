import { ChangeDetectionStrategy, Component, output } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { TxtFileDropDirective } from '@shared/directives';
import { TuiIcon, TuiLink } from '@taiga-ui/core';
import { TuiFiles } from '@taiga-ui/kit';

/**
 * Where a .txt file is dropped (anywhere on the window) or picked. Its
 * height is `--txt-dropzone-height`, 8rem unless set.
 */
@Component({
  selector: 'app-txt-dropzone',
  imports: [FormsModule, TxtFileDropDirective, TuiFiles, TuiIcon, TuiLink],
  templateUrl: './txt-dropzone.component.html',
  styleUrl: './txt-dropzone.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TxtDropzoneComponent {
  /** The text of each file. */
  readonly fileContent = output<string>();

  async onFileSelect(file: File | null) {
    if (file) this.fileContent.emit(await file.text());
  }
}
