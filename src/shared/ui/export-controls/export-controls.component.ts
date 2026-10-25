import { ChangeDetectionStrategy, Component, EventEmitter, Input, Output, signal } from '@angular/core';
import { TuiButton } from '@taiga-ui/core';
import { TuiSegmented } from '@taiga-ui/kit';

export interface ExportFormat {
  value: string;
  label: string;
}

@Component({
  selector: 'app-export-controls',
  imports: [TuiButton, TuiSegmented],
  templateUrl: './export-controls.component.html',
  styleUrl: './export-controls.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExportControlsComponent {
  @Input({ required: true }) formats: ExportFormat[] = [];
  @Input() count = 0;
  @Input() countLabel = 'beatmapsets';
  @Input() disabled = false;
  @Output() doExport = new EventEmitter<string>();

  readonly selectedIndex = signal(0);

  export() {
    const format = this.formats[this.selectedIndex()];
    if (format) this.doExport.emit(format.value);
  }
}
