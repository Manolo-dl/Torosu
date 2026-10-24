import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { UpdateStore } from '@entities/update';
import { NavStore } from '@entities/nav-item';
import { FaIconComponent } from '@fortawesome/angular-fontawesome';
import { faSun, faMoon, faGears, faCircleArrowUp } from '@fortawesome/free-solid-svg-icons';
import { TUI_DARK_MODE, TuiButton, TuiDropdown, TuiTitle } from "@taiga-ui/core";
import { TuiBadge, TuiButtonLoading, TuiProgress } from '@taiga-ui/kit';
import { ConfigDrawerComponent } from "@features/configuration";

@Component({
  selector: 'app-header',
  imports: [FaIconComponent, TuiButton, TuiButtonLoading, TuiDropdown, TuiTitle, TuiBadge, TuiProgress, ConfigDrawerComponent],
  templateUrl: './header.component.html',
  styleUrl: './header.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HeaderComponent {
  readonly darkMode = inject(TUI_DARK_MODE);
  readonly navStore = inject(NavStore);
  readonly icon = computed(() => this.darkMode() ? faMoon : faSun);
  readonly faGears = faGears;
  readonly faCircleArrowUp = faCircleArrowUp;
  readonly update = inject(UpdateStore);
  readonly updateOpen = signal(false);
}
