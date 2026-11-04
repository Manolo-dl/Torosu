import { Component, computed, inject, linkedSignal, model, signal, untracked } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
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
import { OsuPathStore } from '@entities/osu-path';
import { NotifyService } from '@shared/services';
import { PlatformStore } from '@shared/stores';
import { DEFAULT_LANGUAGE, LANGUAGES } from '@shared/i18n';
import { OptionGroupComponent, OptionGroupItem } from '@shared/ui';
import { NameFormatField, NameFormatFieldComponent, NameFormatPreset } from '../name-format-field/name-format-field.component';

/** The names the trainer always gave (`DEFAULT_NAME_FORMAT` in trainer/naming.rs). */
const DEFAULT_NAME_FORMAT = '{diff} {rate} ({bpm}bpm) ({pitch}) {tasks} {stats}';
/** The names the pack creator always gave (`DEFAULT_PACK_NAME_FORMAT` in packs/naming.rs). */
const DEFAULT_PACK_NAME_FORMAT = '[{creator}] {artist} - {title} ({diff})';

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
    name_format: DEFAULT_PACK_NAME_FORMAT,
  },
  trainer: {
    high_quality_audio: false,
    high_quality_video: false,
    auto_pitch: false,
    open_osz: true,
    name_format: DEFAULT_NAME_FORMAT,
    shortcuts_enabled: false,
    shortcuts: [],
  },
  open_file: false,
  dev_mode: false,
};

@Component({
  selector: 'app-config-drawer',
  imports: [FormsModule, TuiDrawer, TuiHeader, TuiTitle, TuiNavComponent, TuiTabsHorizontal, TuiTab, TuiSwitch, TuiInputNumber, TuiCell, TuiInput, TuiLabel, TuiPopup, TuiButton, NameFormatFieldComponent, OptionGroupComponent, TranslocoPipe],
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
  private readonly transloco = inject(TranslocoService);
  /** Global shortcuts don't work on Wayland. */
  readonly platform = inject(PlatformStore);

  /** Opened by the settings button (`ConfigButtonComponent`), which loads the settings first. */
  readonly open = model.required<boolean>();
  readonly activeTab = signal(0);
  readonly tools = USER_ITEMS.filter(item => item.route !== '/');

  /**
   * Local copy edited by the form; sent to the backend on save. Copied again
   * from the saved settings each time the drawer opens, so what was not saved
   * is dropped; but not when those change while it is open. The osu! folder
   * is the one in use, empty without one: it may have been detected or chosen
   * after the settings were read.
   */
  readonly draft = linkedSignal({
    source: this.open,
    computation: () => untracked(() => ({
      ...structuredClone(this.store.config() ?? DEFAULT_CONFIG),
      osu_path: this.osuPath.path() ?? undefined,
    })),
  });

  /** Each language named in itself. */
  readonly languageOptions: OptionGroupItem[] = LANGUAGES.map(language => ({ label: language.name }));

  /** English when none is saved. */
  readonly languageIndex = computed(() =>
    Math.max(0, LANGUAGES.findIndex(language => language.code === (this.draft().language ?? DEFAULT_LANGUAGE))),
  );

  setLanguage(index: number) {
    this.draft.update(draft => ({ ...draft, language: LANGUAGES[index].code }));
  }

  readonly trainerNameFormats: NameFormatPreset[] = [
    { label: 'settings.nameFormat.presets.default', format: DEFAULT_NAME_FORMAT },
    { label: 'settings.nameFormat.presets.rateFirst', format: '[{rate}] {diff} ({pitch}) {tasks} {stats}' },
    { label: 'settings.nameFormat.presets.bpmFirst', format: '[{bpm}bpm] {diff} ({pitch}) {tasks} {stats}' },
  ];

  readonly trainerNameFields: NameFormatField[] = [
    { field: '{diff}', meaning: 'settings.nameFormat.trainer.diff' },
    { field: '{rate}', meaning: 'settings.nameFormat.trainer.rate' },
    { field: '{bpm}', meaning: 'settings.nameFormat.trainer.bpm' },
    { field: '{pitch}', meaning: 'settings.nameFormat.trainer.pitch' },
    { field: '{sv}', meaning: 'settings.nameFormat.trainer.sv' },
    { field: '{keys}', meaning: 'settings.nameFormat.trainer.keys' },
    { field: '{fln}', meaning: 'settings.nameFormat.trainer.fln' },
    { field: '{tasks}', meaning: 'settings.nameFormat.trainer.tasks' },
    { field: '{stats}', meaning: 'settings.nameFormat.trainer.stats' },
  ];

  readonly packNameFormats: NameFormatPreset[] = [
    { label: 'settings.nameFormat.presets.default', format: DEFAULT_PACK_NAME_FORMAT },
    { label: 'settings.nameFormat.presets.starsFirst', format: '[{stars}★] {artist} - {title} ({diff})' },
    { label: 'settings.nameFormat.presets.short', format: '{title} ({diff})' },
  ];

  readonly packNameFields: NameFormatField[] = [
    { field: '{creator}', meaning: 'settings.nameFormat.packs.creator' },
    { field: '{artist}', meaning: 'settings.nameFormat.packs.artist' },
    { field: '{title}', meaning: 'settings.nameFormat.packs.title' },
    { field: '{diff}', meaning: 'settings.nameFormat.packs.diff' },
    { field: '{stars}', meaning: 'settings.nameFormat.packs.stars' },
    { field: '{mode}', meaning: 'settings.nameFormat.packs.mode' },
    { field: '{keys}', meaning: 'settings.nameFormat.packs.keys' },
  ];

  async save() {
    // A new osu! folder is checked first: nothing is saved if it is not osu!'s.
    // Emptied, it is forgotten and the app asks for one again.
    const path = this.draft().osu_path;
    const pathChanged = (path ?? null) !== this.osuPath.path();
    if (pathChanged) {
      try {
        if (path) await this.osuPath.setPath(path);
        else await this.osuPath.clearPath();
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

    // The tool open was just turned off, or has no osu! folder now.
    const url = this.router.url;
    const turnedOff = [...this.store.excludedTools()].some(route => url === route || url.startsWith(`${route}/`));
    if (turnedOff || (!this.osuPath.path() && url.startsWith('/user/'))) {
      await this.router.navigate(['/']);
    }
  }

  // Set with `update`, which redraws the form: after the dialog nothing else
  // would.
  async pickOsuPath() {
    const path = await this.pickFolder(this.transloco.translate('settings.general.pickOsuPath'), this.draft().osu_path);
    if (path) this.draft.update(draft => ({ ...draft, osu_path: path }));
  }

  async pickOutputPath() {
    const path = await this.pickFolder(this.transloco.translate('settings.downloader.pickOutputPath'), this.draft().downloader.output_path);
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
