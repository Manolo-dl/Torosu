import { ButtonComponent } from '../button/button.component';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, EventEmitter, Input, OnChanges, Output, signal } from '@angular/core';
import { OptionGroupComponent, OptionGroupItem } from '../option-group/option-group.component';

export interface ExportFormat {
  value: string;
  /** A translation key. */
  label: string;
}

@Component({
  selector: 'app-export-controls',
  imports: [OptionGroupComponent, TranslocoPipe, ButtonComponent],
  templateUrl: './export-controls.component.html',
  styleUrl: './export-controls.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExportControlsComponent implements OnChanges {
  @Input({ required: true }) formats: ExportFormat[] = [];
  @Input() count = 0;
  @Input() disabled = false;
  @Output() doExport = new EventEmitter<string>();

  readonly selectedIndex = signal(0);

  /** The formats, for the option group. */
  options: OptionGroupItem[] = [];

  ngOnChanges() {
    this.options = this.formats.map(format => ({ label: format.label, translate: true }));
  }

  export() {
    const format = this.formats[this.selectedIndex()];
    if (format) this.doExport.emit(format.value);
  }
}
