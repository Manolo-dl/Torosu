import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { TuiTitle } from '@taiga-ui/core';

/** A page's title and subtitle; what the page puts inside goes on their right. */
@Component({
  selector: 'app-page-header',
  imports: [TuiTitle],
  templateUrl: './page-header.component.html',
  styleUrl: './page-header.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PageHeaderComponent {
  // Not `title`: as an attribute it would also be the tooltip of the header.
  readonly heading = input.required<string>();
  readonly subtitle = input<string>();
}
