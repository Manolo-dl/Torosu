import { Component } from '@angular/core';
import { PackSearchInputComponent, PackBrowserComponent, PackPreviewComponent, PackDetailsFormComponent, CreatePackButtonComponent, PackCollectionAdderComponent } from "@features/pack-creator";

@Component({
  selector: 'app-create-packs',
  imports: [PackSearchInputComponent, PackBrowserComponent, PackPreviewComponent, PackDetailsFormComponent, CreatePackButtonComponent, PackCollectionAdderComponent],
  templateUrl: './create-packs-page.component.html',
  styleUrl: './create-packs-page.component.css',
})
export class CreatePacksPageComponent {}
