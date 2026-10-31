import { ChangeDetectionStrategy, Component, computed, effect, inject, signal, untracked } from '@angular/core';
import { Router } from '@angular/router';
import { ConfigurationStore } from '@entities/configuration';
import { UpdateStore } from '@entities/update';
import { NavStore } from '@entities/nav-item';
import { TUI_DARK_MODE, TuiButton, TuiDropdown, TuiTitle } from "@taiga-ui/core";
import { TuiBadge, TuiButtonLoading, TuiProgress } from '@taiga-ui/kit';
import { ConfigButtonComponent } from "@features/configuration";

@Component({
  selector: 'app-header',
  imports: [TuiButton, TuiButtonLoading, TuiDropdown, TuiTitle, TuiBadge, TuiProgress, ConfigButtonComponent],
  templateUrl: './header.component.html',
  styleUrl: './header.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HeaderComponent {
  readonly darkMode = inject(TUI_DARK_MODE);
  readonly navStore = inject(NavStore);
  readonly icon = computed(() => this.darkMode() ? '@tui.moon' : '@tui.sun');
  readonly update = inject(UpdateStore);
  readonly updateOpen = signal(false);
  private readonly config = inject(ConfigurationStore);
  private readonly router = inject(Router);

  /** Developer mode is turned on in the settings. */
  readonly devMode = computed(() => this.config.config()?.dev_mode ?? false);

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
