import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { APP_ITEMS, DEV_ITEMS, NavItem, NavStore, USER_ITEMS } from '@entities/nav-item';
import { RouterLink, RouterLinkActive } from "@angular/router";
import { FaIconComponent } from "@fortawesome/angular-fontawesome";
import { faClockRotateLeft } from '@fortawesome/free-solid-svg-icons'
import { getVersion } from "@tauri-apps/api/app";
import { ConfigurationStore } from '@entities/configuration';

@Component({
  selector: 'app-sidebar',
  imports: [RouterLink, RouterLinkActive, FaIconComponent],
  templateUrl: './sidebar.component.html',
  styleUrl: './sidebar.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SidebarComponent {

  readonly navStore = inject(NavStore);
  private readonly config = inject(ConfigurationStore);
  readonly version = getVersion()
  readonly faClockRotateLeft = faClockRotateLeft;

  /** The tools, without the ones turned off in the configuration. */
  readonly userItems = computed<NavItem[]>(() => USER_ITEMS.filter(item => !this.config.excludedTools().has(item.route)));
  readonly devItems: NavItem[] = DEV_ITEMS;
  readonly appItems: NavItem[] = APP_ITEMS;
  /** Changelog, Github, Issues and Support: hidden for now, they have no pages yet. */
  readonly showAppItems = false;
}
