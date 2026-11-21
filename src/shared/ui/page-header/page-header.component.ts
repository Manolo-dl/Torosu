import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { IconComponent } from '../icon/icon.component';

/**
 * A page's title, with the icon of its tool and a subtitle; what the page
 * puts inside goes on their right.
 */
@Component({
  selector: 'app-page-header',
  imports: [IconComponent],
  templateUrl: './page-header.component.html',
  styleUrl: './page-header.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PageHeaderComponent {
  // Not `title`: as an attribute it would also be the tooltip of the header.
  readonly heading = input.required<string>();
  readonly subtitle = input<string>();
  /** The tool's icon, as in the sidebar. */
  readonly icon = input<string>();

  readonly transitionName = computed(() => toolTransitionName(this.icon()));
}

/**
 * The View Transition name of a tool's icon, the same on its card on the home
 * and on its page's header, so it flies from one to the other.
 */
export function toolTransitionName(icon: string | undefined): string | null {
  return icon ? `tool-${icon.replace(/^@tui\./, '')}` : null;
}
