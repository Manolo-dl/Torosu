import { BadgeComponent, CardComponent, LoaderComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { CollectionStore } from '@entities/collection';
import { TuiCell, TuiCheckbox, TuiLabel, TuiScrollbar, TuiTitle } from '@taiga-ui/core';

@Component({
  selector: 'app-collection-list',
  imports: [LoaderComponent, FormsModule, TuiCell, TuiCheckbox, TuiLabel, TuiScrollbar, TuiTitle, TranslocoPipe, BadgeComponent, CardComponent],
  templateUrl: './collection-list.component.html',
  styleUrl: './collection-list.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionListComponent {
  readonly store = inject(CollectionStore);
}
