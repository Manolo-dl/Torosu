import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { getVersion } from '@tauri-apps/api/app';
import { ConfigurationStore } from '@entities/configuration';
import { USER_ITEMS } from '@entities/nav-item';
import { OsuPathStore } from '@shared/stores';
import { TuiIcon, TuiTitle } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

/** What each tool does, by route. */
const DESCRIPTIONS: Record<string, string> = {
  '/user/downloader': 'Download beatmap sets from a list of links straight into your Songs folder, skipping the ones you have.',
  '/user/export-collections': 'Save your collections as beatmap links, or as a list to import on another install.',
  '/user/import-collections': 'Bring collections from a file into osu!, merged with the ones you already have.',
  '/user/export-song-folder': 'Filter your library by mode, status, stars, length and more, and export it as beatmap links.',
  '/user/create-packs': 'Put diffs from any sets, rates and unsubmitted maps too, into a pack of your own.',
  '/user/trainer': 'Change the rate, the stats, the long notes or the key count of the map selected in osu!.',
};

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
    USER_ITEMS.filter(item => item.route !== '/' && !this.config.excludedTools().has(item.route)).map(item => ({
      ...item,
      description: DESCRIPTIONS[item.route] ?? '',
    })),
  );

  constructor() {
    getVersion().then(version => this.version.set(version)).catch(() => {});
  }
}
