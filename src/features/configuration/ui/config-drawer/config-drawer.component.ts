import { Component, inject, linkedSignal, model, signal, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { TuiDrawer, TuiTabsHorizontal, TuiTab, TuiSwitch, TuiInputNumber } from "@taiga-ui/kit";
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { TuiHeader, TuiNavComponent } from "@taiga-ui/layout";
import { TuiTitle, TuiPopup, TuiButton, TuiCell, TuiInput, TuiLabel } from "@taiga-ui/core";
import { Router } from '@angular/router';
import { AppConfig, ConfigurationStore } from '@entities/configuration';
import { USER_ITEMS } from '@entities/nav-item';
import { OsuDbStore } from '@entities/osu-db';
import { CollectionStore } from '@entities/collection';
import { OsuPathStore } from '@shared/stores';
import { NotifyService } from '@shared/services';

// Mirrors the serde defaults in src-tauri/src/commands/config/model.rs
const DEFAULT_CONFIG: AppConfig = {
  excluded_tools: [],
  downloader: {
    notify: false,
    error_file: true,
    save_session: false,
    load_library: false,
    skip_video: false,
    max_concurrent_downloads: 1,
  },
  packs: {
    keep_background: true,
    max_sets_per_pack: 0,
  },
  open_file: false,
  dev_mode: false,
};

@Component({
  selector: 'app-config-drawer',
  imports: [FormsModule, TuiDrawer, TuiHeader, TuiTitle, TuiNavComponent, TuiTabsHorizontal, TuiTab, TuiSwitch, TuiInputNumber, TuiCell, TuiInput, TuiLabel, TuiPopup, TuiButton],
  templateUrl: './config-drawer.component.html',
  styleUrl: './config-drawer.component.less',
})
export class ConfigDrawerComponent {
  private readonly store = inject(ConfigurationStore);
  private readonly router = inject(Router);
  private readonly osuPath = inject(OsuPathStore);
  private readonly osuDb = inject(OsuDbStore);
  private readonly collections = inject(CollectionStore);
  private readonly notify = inject(NotifyService);

  /** Opened by the settings button (`ConfigButtonComponent`), which loads the settings first. */
  readonly open = model.required<boolean>();
  readonly activeTab = signal(0);
  readonly tools = USER_ITEMS.filter(item => item.route !== '/');

  /**
   * Local copy edited by the form; sent to the backend on save. Copied again
   * from the saved settings each time the drawer opens, so what was not saved
   * is dropped; but not when those change while it is open.
   */
  readonly draft = linkedSignal({
    source: this.open,
    computation: () => untracked(() => structuredClone(this.store.config() ?? DEFAULT_CONFIG)),
  });

  async save() {
    // A new osu! folder is checked first: nothing is saved if it is not osu!'s.
    const path = this.draft().osu_path;
    const pathChanged = !!path && path !== this.store.config()?.osu_path;
    if (pathChanged) {
      try {
        await this.osuPath.setPath(path);
      } catch (e) {
        this.notify.error(String(e));
        return;
      }
    }

    await this.store.updateConfig(this.draft());
    this.open.set(false);

    // The library and collections loaded are the old folder's.
    if (pathChanged) {
      this.osuDb.reset();
      this.collections.reset();
    }

    // The tool open was just turned off.
    const url = this.router.url;
    if ([...this.store.excludedTools()].some(route => url === route || url.startsWith(`${route}/`))) {
      await this.router.navigate(['/']);
    }
  }

  // Set with `update`, which redraws the form: after the dialog nothing else
  // would.
  async pickOsuPath() {
    const path = await this.pickFolder('Select your osu! folder', this.draft().osu_path);
    if (path) this.draft.update(draft => ({ ...draft, osu_path: path }));
  }

  async pickOutputPath() {
    const path = await this.pickFolder('Select the download folder', this.draft().downloader.output_path);
    if (path) this.draft.update(draft => ({ ...draft, downloader: { ...draft.downloader, output_path: path } }));
  }

  private async pickFolder(title: string, defaultPath?: string): Promise<string | null> {
    return openDialog({ directory: true, multiple: false, title, defaultPath });
  }

  isToolEnabled(route: string): boolean {
    return !this.draft().excluded_tools.includes(route);
  }

  setToolEnabled(route: string, enabled: boolean) {
    this.draft.update(draft => {
      const excluded = draft.excluded_tools.filter(r => r !== route);
      return { ...draft, excluded_tools: enabled ? excluded : [...excluded, route] };
    });
  }
}
