import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { CollectionStore } from '@entities/collection';
import { TuiCell, TuiCheckbox, TuiLabel, TuiLoader, TuiScrollbar, TuiTitle } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-collection-list',
  imports: [FormsModule, TuiCardLarge, TuiCell, TuiCheckbox, TuiLabel, TuiLoader, TuiScrollbar, TuiTitle, TuiBadge],
  templateUrl: './collection-list.component.html',
  styleUrl: './collection-list.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionListComponent {
  readonly store = inject(CollectionStore);
}
