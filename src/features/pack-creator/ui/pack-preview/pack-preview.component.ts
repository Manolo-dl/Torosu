import { Component, inject } from '@angular/core';
import { PackStore, SelectedDiff } from '@entities/pack';
import { PackPreviewCardComponent } from "../pack-preview-card/pack-preview-card.component";

@Component({
  selector: 'app-pack-preview',
  imports: [PackPreviewCardComponent],
  templateUrl: './pack-preview.component.html',
  styleUrl: './pack-preview.component.css',
})
export class PackPreviewComponent {

  readonly store = inject(PackStore);

  trackDiff(_index: number, diff: SelectedDiff): string {
    return diff.md5;
  }
}
