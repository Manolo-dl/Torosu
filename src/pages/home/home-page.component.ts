import { ChangeDetectionStrategy, Component } from '@angular/core';
import { TuiTitle, TuiNotificationDirective, TuiButton, TuiAppearance } from "@taiga-ui/core";
import { TuiSurface } from '@taiga-ui/layout'
import { FaIconComponent } from "@fortawesome/angular-fontawesome";

@Component({
  selector: 'app-home',
  imports: [TuiTitle, TuiNotificationDirective, TuiButton, TuiSurface, TuiAppearance],
  templateUrl: './home-page.component.html',
  styleUrl: './home-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HomePageComponent {
}
