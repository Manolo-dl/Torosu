import { BadgeComponent, ButtonComponent, IconComponent, ProgressComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ApplicationRef, ChangeDetectionStrategy, Component, computed, DOCUMENT, effect, inject, signal, untracked } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { ConfigurationStore } from '@entities/configuration';
import { UpdateStore } from '@entities/update';
import { NavStore } from '@entities/nav-item';
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
      startViewTransition?: (update: () => void) => { ready: Promise<void> };
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
    const transition = document.startViewTransition(() => {
      this.appearance.toggleDark();
      // The new theme on the page before the transition takes its picture.
      this.appRef.tick();
    });
    transition.ready
      .then(() =>
        document.documentElement.animate(
          { clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${radius}px at ${x}px ${y}px)`] },
          { duration: 520, easing: 'cubic-bezier(0.65, 0, 0.35, 1)', pseudoElement: '::view-transition-new(root)' },
        ),
      )
      .catch(() => {});
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
