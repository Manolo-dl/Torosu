import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { NavigationEnd, Router, RouterLink, RouterLinkActive } from '@angular/router';
import { filter, map, startWith } from 'rxjs';
import { getVersion } from '@tauri-apps/api/app';
import { APP_ITEMS, DEV_ITEMS, NavItem, NavStore, USER_ITEMS } from '@entities/nav-item';
import { ConfigurationStore } from '@entities/configuration';
import { IconComponent } from '@shared/ui';

@Component({
  selector: 'app-sidebar',
  imports: [IconComponent, RouterLink, RouterLinkActive, TranslocoPipe],
  templateUrl: './sidebar.component.html',
  styleUrl: './sidebar.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SidebarComponent {
  readonly navStore = inject(NavStore);
  private readonly config = inject(ConfigurationStore);
  private readonly router = inject(Router);

  readonly version = signal<string | null>(null);

  constructor() {
    getVersion().then(version => this.version.set(version)).catch(() => {});
  }

  /** The tools, without the ones turned off in the configuration. */
  readonly userItems = computed<NavItem[]>(() => USER_ITEMS.filter(item => !this.config.excludedTools().has(item.route)));
  readonly devItems: NavItem[] = DEV_ITEMS;
  readonly appItems: NavItem[] = APP_ITEMS;
  /** Changelog, Github, Issues and Support: hidden for now, they have no pages yet. */
  readonly showAppItems = false;

  readonly items = computed(() => (this.navStore.isUserMode() ? this.userItems() : this.devItems));

  private readonly url = toSignal(
    this.router.events.pipe(
      filter(event => event instanceof NavigationEnd),
      map(() => this.router.url),
      startWith(this.router.url),
    ),
    { initialValue: this.router.url },
  );

  /** Where the indicator sits: the item of the page open, -1 for none. */
  readonly activeIndex = computed(() => {
    const url = this.url().split(/[?#]/)[0];
    return this.items().findIndex(item => (item.route === '/' ? url === '/' : url === item.route || url.startsWith(item.route + '/')));
  });
}
