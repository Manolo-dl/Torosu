import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { getVersion } from '@tauri-apps/api/app';
import { ConfigurationStore } from '@entities/configuration';
import { USER_ITEMS } from '@entities/nav-item';
import { OsuPathStore } from '@entities/osu-path';
import { BadgeComponent, CardComponent, IconComponent, ModeOrbitComponent, TrianglesComponent } from '@shared/ui';
import { MusicPulseDirective, OsuReaderStore } from '@entities/osu-reader';
import { HomeLibraryComponent } from './ui/home-library.component';
import { HomeSelectedComponent } from './ui/home-selected.component';
import { HomeWordmarkComponent } from './ui/home-wordmark.component';

@Component({
  selector: 'app-home',
  imports: [BadgeComponent, CardComponent, HomeLibraryComponent, HomeSelectedComponent, HomeWordmarkComponent, IconComponent, ModeOrbitComponent, MusicPulseDirective, RouterLink, TrianglesComponent, TranslocoPipe],
  templateUrl: './home-page.component.html',
  styleUrl: './home-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HomePageComponent {
  private readonly config = inject(ConfigurationStore);
  readonly osuPath = inject(OsuPathStore);
  /** The map selected in osu!, if a tool has started the reader. */
  readonly reader = inject(OsuReaderStore);
  readonly version = signal<string | null>(null);

  /** The tools turned on in the settings. */
  readonly tools = computed(() =>
    USER_ITEMS.filter(item => item.route !== '/' && !this.config.excludedTools().has(item.route)),
  );

  constructor() {
    getVersion().then(version => this.version.set(version)).catch(() => {});
  }
}
