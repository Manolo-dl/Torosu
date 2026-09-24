import { ChangeDetectionStrategy, Component, EventEmitter, Input, OnInit, Output, signal } from '@angular/core';

export interface ExportFormat {
  value: string;
  label: string;
}

@Component({
  selector: 'app-export-controls',
  imports: [],
  templateUrl: './export-controls.component.html',
  styleUrl: './export-controls.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExportControlsComponent implements OnInit {
  @Input({ required: true }) formats: ExportFormat[] = [];
  @Input() count = 0;
  @Input() countLabel = 'beatmapsets';
  @Input() disabled = false;
  @Output() doExport = new EventEmitter<string>();

  readonly selectedFormat = signal('');

  ngOnInit() {
    if (this.formats.length > 0) {
      this.selectedFormat.set(this.formats[0].value);
    }
  }

  export() {
    this.doExport.emit(this.selectedFormat());
  }
}
