import { ChangeDetectionStrategy, Component, inject, OnInit } from '@angular/core';
import { BeatmapImportComponent, BeatmapQueueComponent } from "@features/beatmap-search";
import { OsuCookieInputComponent } from '@features/downloader';
import { OsuDbStore } from '@entities/osu-db';
import { ConfigurationStore } from '@entities/configuration';
import { UserStore } from '@entities/user';
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
  imports: [PageHeaderComponent, BeatmapImportComponent, BeatmapQueueComponent, OsuCookieInputComponent, TuiTitle, TuiNotificationDirective, TuiButton, TuiAccordion],
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

  readonly downloaderInfo: PageInfo[] = [
    {
      title: 'How to use the downloader',
      steps: [
        {
          desc: 'Get the osu session cookie (check how to get osu session step)'
        },
        {
          desc: 'Put the osu session in the input'
        },
        {
          desc: 'Drop or select a txt file containing all download links (check file format step)'
        },
        {
          desc: 'In the queue there will be a list with all the map IDs click start'
        }
      ]
    },
    {
      title: 'How to get osu session',
      steps: [
        {
          desc: 'Go to your browser',
        },
        {
          desc: 'Go to the osu page, make sure you are logged'
        },
        {
          desc: 'Right click and choose inspect or press F12',
          image: 'assets/images/downloader/step_3.png'
        },
        {
          desc: 'Go to Application or Storage tab',
          image: 'assets/images/downloader/step_4.png'
        },
        {
          desc: 'Go to cookies -> https://osu.ppy.sh and copy the value of the cookie with the name of osu_session',
          image: 'assets/images/downloader/step_5.png',
        }
      ]
    },
    {
      title: 'File format',
      steps: [
        {
          desc: 'The file must be a txt file with one download link per line',
          image: 'assets/images/downloader/step_6.png'
        }
      ]
    },
    {
      title: 'Configuration',
      steps: [
        {
          desc: 'To go to the configuration click on the gear icon',
          image: 'assets/images/downloader/step_7.png'
        },
        {
          desc: 'Notify: If enabled, a notification will be shown when the download is finished',
        },
        {
          desc: 'Error file: If enabled, a file with the failed downloads will be created in the output folder',
        },
        {
          desc: 'Save cookie: If enabled, the osu session cookie is kept when Torosu closes; otherwise it only lasts for the session',
        },
        {
          desc: 'Output path: The path where the downloaded files will be saved, if not set, the default osu! songs folder will be used',
        },
        {
          desc: 'Load library: If enabled, the osu! library is loaded when you open the downloader, this is useful for checking if the map is already downloaded',
        },
        {
          desc: 'Skip video: If enabled, the video will not be downloaded, this is useful for saving bandwidth and disk space',
        },
        {
          desc: 'Max concurrent downloads: The maximum number of concurrent downloads, default is 1, increase at your own risk',
        }
      ]
    }
  ]
}
