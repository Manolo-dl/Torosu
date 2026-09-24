import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { APP_ITEMS, DEV_ITEMS, NavItem, NavStore, USER_ITEMS } from '@entities/nav-item';
import { RouterLink, RouterLinkActive } from "@angular/router";
import { FaIconComponent } from "@fortawesome/angular-fontawesome";
import { faClockRotateLeft } from '@fortawesome/free-solid-svg-icons'
import { getVersion } from "@tauri-apps/api/app";

@Component({
  selector: 'app-sidebar',
  imports: [RouterLink, RouterLinkActive, FaIconComponent],
  templateUrl: './sidebar.component.html',
  styleUrl: './sidebar.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SidebarComponent {

  readonly navStore = inject(NavStore);
  readonly version = getVersion()
  readonly faClockRotateLeft = faClockRotateLeft;

  readonly userItems: NavItem[] = USER_ITEMS;
  readonly devItems: NavItem[] = DEV_ITEMS;
  readonly appItems: NavItem[] = APP_ITEMS;
}
