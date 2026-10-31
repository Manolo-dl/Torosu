import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { getVersion } from '@tauri-apps/api/app';
import { ConfigurationStore } from '@entities/configuration';
import { USER_ITEMS } from '@entities/nav-item';
import { OsuPathStore } from '@entities/osu-path';
import { TuiIcon, TuiTitle } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-home',
  imports: [RouterLink, TuiIcon, TuiTitle, TuiBadge, TuiCardLarge],
  templateUrl: './home-page.component.html',
  styleUrl: './home-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HomePageComponent {
  private readonly config = inject(ConfigurationStore);
  readonly osuPath = inject(OsuPathStore);
  readonly version = signal<string | null>(null);

  /** The tools turned on in the settings. */
  readonly tools = computed(() =>
    USER_ITEMS.filter(item => item.route !== '/' && !this.config.excludedTools().has(item.route)),
  );

  constructor() {
    getVersion().then(version => this.version.set(version)).catch(() => {});
  }
}
