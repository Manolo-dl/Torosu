import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { TuiDrawer, TuiTabsHorizontal, TuiTab, TuiSwitch, TuiInputNumber } from "@taiga-ui/kit";
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { TuiHeader, TuiNavComponent } from "@taiga-ui/layout";
import { TuiTitle, TuiPopup, TuiButton, TuiCell, TuiInput, TuiLabel } from "@taiga-ui/core";
import { FaIconComponent } from "@fortawesome/angular-fontawesome";
import { faGears } from '@fortawesome/free-solid-svg-icons';
import { AppConfig, ConfigurationStore } from '@entities/configuration';
import { USER_ITEMS } from '@entities/nav-item';

// Mirrors the serde defaults in src-tauri/src/commands/config/model.rs
const DEFAULT_CONFIG: AppConfig = {
  excluded_tools: [],
  tosu: { auto_start: true, kill_on_exit: true },
  downloader: {
    notify: false,
    error_file: true,
    load_library: false,
    skip_video: false,
    max_concurrent_downloads: 1,
  },
  sqlite_db: true,
  open_file: false,
};

@Component({
  selector: 'app-config-drawer',
  imports: [FormsModule, TuiDrawer, TuiHeader, TuiTitle, TuiNavComponent, TuiTabsHorizontal, TuiTab, TuiSwitch, TuiInputNumber, TuiCell, TuiInput, TuiLabel, FaIconComponent, TuiPopup, TuiButton],
  templateUrl: './config-drawer.component.html',
  styleUrl: './config-drawer.component.less',
})
export class ConfigDrawerComponent {
  private readonly store = inject(ConfigurationStore);

  readonly open = signal(false);
  readonly activeTab = signal(0);
  readonly faGears = faGears;
  readonly tools = USER_ITEMS.filter(item => item.route !== '/');

  // Local copy edited by the form; not sent to the backend yet
  draft: AppConfig = structuredClone(DEFAULT_CONFIG);

  async toggle() {
    if (this.open()) {
      this.open.set(false);
      return;
    }

    if (!this.store.isLoaded()) await this.store.loadConfig();
    this.draft = structuredClone(this.store.config() ?? DEFAULT_CONFIG);
    this.open.set(true);
  }

  async pickOsuPath() {
    const path = await this.pickFolder('Select your osu! folder', this.draft.osu_path);
    if (path) this.draft.osu_path = path;
  }

  async pickOutputPath() {
    const path = await this.pickFolder('Select the download folder', this.draft.downloader.output_path);
    if (path) this.draft.downloader.output_path = path;
  }

  private async pickFolder(title: string, defaultPath?: string): Promise<string | null> {
    return openDialog({ directory: true, multiple: false, title, defaultPath });
  }

  isToolEnabled(route: string): boolean {
    return !this.draft.excluded_tools.includes(route);
  }

  setToolEnabled(route: string, enabled: boolean) {
    const excluded = this.draft.excluded_tools.filter(r => r !== route);
    this.draft.excluded_tools = enabled ? excluded : [...excluded, route];
  }
}
