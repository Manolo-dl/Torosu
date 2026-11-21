import { BadgeComponent, ButtonComponent, IconComponent, ProgressComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ApplicationRef, ChangeDetectionStrategy, Component, computed, DOCUMENT, effect, inject, signal, untracked } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { ConfigurationStore } from '@entities/configuration';
import { UpdateStore } from '@entities/update';
import { DEV_ITEMS, NavStore } from '@entities/nav-item';
import { TUI_DARK_MODE, TuiDropdown, TuiTitle } from "@taiga-ui/core";
import { AppearanceStore } from "@shared/stores";
import { WindowControlsComponent } from "@widgets/window-controls/window-controls.component";
import { ConfigButtonComponent } from "@features/configuration";

@Component({
  selector: 'app-header',
  imports: [ProgressComponent, ButtonComponent, IconComponent, RouterLink, WindowControlsComponent, TuiDropdown, TuiTitle, ConfigButtonComponent, TranslocoPipe, BadgeComponent],
  templateUrl: './header.component.html',
  styleUrl: './header.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HeaderComponent {
  readonly darkMode = inject(TUI_DARK_MODE);
  readonly appearance = inject(AppearanceStore);
  readonly navStore = inject(NavStore);
  readonly icon = computed(() => this.appearance.dark() ? 'moon' : 'sun');
  readonly update = inject(UpdateStore);
  readonly updateOpen = signal(false);
  private readonly config = inject(ConfigurationStore);
  private readonly router = inject(Router);
  private readonly document = inject(DOCUMENT);
  private readonly appRef = inject(ApplicationRef);

  /** Developer mode is turned on in the settings. */
  readonly devMode = computed(() => this.config.config()?.dev_mode ?? false);

  /**
   * The other theme, spreading in a circle from the button where the engine
   * can (View Transitions), at once where it can't or motion is reduced.
   */
  toggleTheme(event: MouseEvent) {
    const document = this.document as Document & {
      startViewTransition?: (update: () => void) => { ready: Promise<void>; finished: Promise<void> };
    };
    const reduced =
      this.appearance.reducedMotion() || matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (!document.startViewTransition || reduced) {
      this.appearance.toggleDark();
      return;
    }

    const x = event.clientX;
    const y = event.clientY;
    const radius = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y));
    // The page's panel is a picture of its own (its view-transition-name,
    // for the pages' transitions): the same circle is drawn on it, in its
    // own coordinates. Its name is never taken away for this: WebKitGTK then
    // drew parts of the page out of place after the transition.
    const panel = document.querySelector('.page-outlet')?.getBoundingClientRect();
    const root = document.documentElement;
    root.classList.add('theme-transition');
    const transition = document.startViewTransition(() => {
      this.appearance.toggleDark();
      // The new theme on the page before the transition takes its picture.
      this.appRef.tick();
    });
    const reveal = { duration: 520, easing: 'cubic-bezier(0.65, 0, 0.35, 1)' };
    const circle = (cx: number, cy: number) => [`circle(0px at ${cx}px ${cy}px)`, `circle(${radius}px at ${cx}px ${cy}px)`];
    transition.ready
      .then(() => {
        root.animate({ clipPath: circle(x, y) }, { ...reveal, pseudoElement: '::view-transition-new(root)' });
        if (panel) {
          root.animate(
            { clipPath: circle(x - panel.left, y - panel.top) },
            { ...reveal, pseudoElement: '::view-transition-new(page)' },
          );
        }
      })
      .catch(() => {});
    transition.finished.finally(() => root.classList.remove('theme-transition'));
  }

  /** From the tools to the developer pages and back, opening the first of them. */
  switchMode() {
    this.navStore.toggleMode();
    this.router.navigate([this.navStore.isDevMode() ? DEV_ITEMS[0].route : '/']);
  }

  constructor() {
    // Turned off while on the developer pages: back to the tools.
    effect(() => {
      if (this.devMode()) return;
      untracked(() => {
        this.navStore.userMode();
        if (this.router.url.startsWith('/dev')) this.router.navigate(['/']);
      });
    });
  }
}
