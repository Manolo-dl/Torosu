import { Component, signal } from '@angular/core';
import { TuiDrawer, TuiTabsHorizontal, TuiTab } from "@taiga-ui/kit";
import { TuiHeader, TuiNavComponent } from "@taiga-ui/layout";
import { TuiTitle, TuiPopup, TuiButton } from "@taiga-ui/core";
import { FaIconComponent } from "@fortawesome/angular-fontawesome";
import { faGears } from '@fortawesome/free-solid-svg-icons';

@Component({
  selector: 'app-config-drawer',
  imports: [TuiDrawer, TuiHeader, TuiTitle, TuiNavComponent, TuiTabsHorizontal, TuiTab, FaIconComponent, TuiPopup, TuiButton],
  templateUrl: './config-drawer.component.html',
  styleUrl: './config-drawer.component.less',
})
export class ConfigDrawerComponent {
  readonly open = signal(false);
  readonly faGears = faGears;
}
