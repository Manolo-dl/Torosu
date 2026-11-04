import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject, OnInit } from '@angular/core';
import { BeatmapImportComponent, BeatmapQueueComponent, OsuCookieInputComponent, UserStore } from '@features/downloader';
import { OsuDbStore } from '@entities/osu-db';
import { ConfigurationStore } from '@entities/configuration';
import { TuiTitle, TuiNotificationDirective, TuiButton } from "@taiga-ui/core";
import { PageHeaderComponent } from '@shared/ui';
import { TuiAccordion } from '@taiga-ui/kit';

interface PageInfo {
  title: string,
  steps: Step[]
}

interface Step {
  desc: string,
  image?: string
}

@Component({
  selector: 'app-downloader',
  imports: [PageHeaderComponent, BeatmapImportComponent, BeatmapQueueComponent, OsuCookieInputComponent, TuiTitle, TuiNotificationDirective, TuiButton, TuiAccordion, TranslocoPipe],
  templateUrl: './downloader-page.component.html',
  styleUrl: './downloader-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DownloaderPageComponent implements OnInit {
  readonly osuDb = inject(OsuDbStore);
  readonly userStore = inject(UserStore);
  private readonly config = inject(ConfigurationStore);

  /** With `load_library` on, the library loads by itself to skip maps already owned. */
  async ngOnInit() {
    await this.config.ensureLoaded();
    if (this.config.config()?.downloader.load_library) await this.osuDb.ensureLoaded();
  }

  async loadLibrary() {
    await this.osuDb.load();
  }

  /** Titles and steps are translation keys. */
  readonly downloaderInfo: PageInfo[] = [
    {
      title: 'downloader.guide.howTo',
      steps: [
        { desc: 'downloader.guide.howTo1' },
        { desc: 'downloader.guide.howTo2' },
        { desc: 'downloader.guide.howTo3' },
        { desc: 'downloader.guide.howTo4' },
      ],
    },
    {
      title: 'downloader.guide.session',
      steps: [
        { desc: 'downloader.guide.session1' },
        { desc: 'downloader.guide.session2' },
        { desc: 'downloader.guide.session3', image: 'assets/images/downloader/step_3.png' },
        { desc: 'downloader.guide.session4', image: 'assets/images/downloader/step_4.png' },
        { desc: 'downloader.guide.session5', image: 'assets/images/downloader/step_5.png' },
      ],
    },
    {
      title: 'downloader.guide.format',
      steps: [
        { desc: 'downloader.guide.format1', image: 'assets/images/downloader/step_6.png' },
      ],
    },
    {
      title: 'downloader.guide.settings',
      steps: [
        { desc: 'downloader.guide.settings1', image: 'assets/images/downloader/step_7.png' },
        { desc: 'downloader.guide.settings2' },
        { desc: 'downloader.guide.settings3' },
        { desc: 'downloader.guide.settings4' },
        { desc: 'downloader.guide.settings5' },
        { desc: 'downloader.guide.settings6' },
        { desc: 'downloader.guide.settings7' },
        { desc: 'downloader.guide.settings8' },
      ],
    },
  ];
}
