import { ChangeDetectionStrategy, Component } from '@angular/core';
import { PackSearchInputComponent, PackBrowserComponent, PackPreviewComponent, PackDetailsFormComponent, CreatePackButtonComponent, PackCollectionAdderComponent } from "@features/pack-creator";
import { PageHeaderComponent } from '@shared/ui';

@Component({
  selector: 'app-create-packs',
  imports: [PackSearchInputComponent, PackBrowserComponent, PackPreviewComponent, PackDetailsFormComponent, CreatePackButtonComponent, PackCollectionAdderComponent, PageHeaderComponent],
  templateUrl: './create-packs-page.component.html',
  styleUrl: './create-packs-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CreatePacksPageComponent {}
