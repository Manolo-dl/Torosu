import { ChangeDetectionStrategy, Component } from '@angular/core';
import { PackSearchInputComponent, PackBrowserComponent, PackPreviewComponent, PackDetailsFormComponent, CreatePackButtonComponent, PackCollectionAdderComponent } from "@features/pack-creator";
import { TuiTitle } from '@taiga-ui/core';

@Component({
  selector: 'app-create-packs',
  imports: [PackSearchInputComponent, PackBrowserComponent, PackPreviewComponent, PackDetailsFormComponent, CreatePackButtonComponent, PackCollectionAdderComponent, TuiTitle],
  templateUrl: './create-packs-page.component.html',
  styleUrl: './create-packs-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CreatePacksPageComponent {}
