import { Component, computed, inject, linkedSignal, model, signal, untracked } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { FormsModule } from '@angular/forms';
import { TuiInputNumber } from "@taiga-ui/kit";
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { TuiTitle, TuiCell, TuiInput, TuiLabel } from "@taiga-ui/core";
import { Router } from '@angular/router';
import { AppConfig, ConfigurationStore } from '@entities/configuration';
import { USER_ITEMS } from '@entities/nav-item';
import { OsuDbStore } from '@entities/osu-db';
import { CollectionStore } from '@entities/collection';
import { OsuPathStore } from '@entities/osu-path';
import { NotifyService } from '@shared/services';
import { PlatformStore } from '@shared/stores';
import { DEFAULT_LANGUAGE, errorText, LANGUAGES } from '@shared/i18n';
import { ButtonComponent, DrawerComponent, IconComponent, OptionGroupComponent, OptionGroupItem, SectionTitleComponent, SwitchComponent } from '@shared/ui';
import { NameFormatField, NameFormatFieldComponent, NameFormatPreset } from '../name-format-field/name-format-field.component';
import { AppearanceSettingsComponent } from '../appearance-settings/appearance-settings.component';
import { PerformanceSettingsComponent } from '../performance-settings/performance-settings.component';

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
    skill_display: 'dan',
    include_mods: false,
    shortcuts_enabled: false,
    shortcuts: [],
  },
  replays: {
    follow: true,
    key_chart: true,
    histogram: true,
    timeline: true,
  },
  open_file: false,
  dev_mode: false,
};

/** The settings as text to compare, without the osu! folder (saved apart), keys in order. */
function comparable(config: AppConfig): string {
  const { osu_path: _, ...settings } = config;
  return JSON.stringify(settings, (_key, value) =>
    value && typeof value === 'object' && !Array.isArray(value)
      ? Object.fromEntries(Object.entries(value).sort(([a], [b]) => a.localeCompare(b)))
      : value,
  );
}

@Component({
  selector: 'app-config-drawer',
  imports: [DrawerComponent, PerformanceSettingsComponent, IconComponent, SectionTitleComponent, SwitchComponent, AppearanceSettingsComponent, FormsModule, TuiTitle, TuiInputNumber, TuiCell, TuiInput, TuiLabel, NameFormatFieldComponent, OptionGroupComponent, TranslocoPipe, ButtonComponent],
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
  /** The sections of the settings, in the nav on the drawer's left. */
  readonly tabs: OptionGroupItem[] = [
    { tab: 'general', icon: 'settings-2' },
    { tab: 'appearance', icon: 'palette' },
    { tab: 'performance', icon: 'gauge' },
    { tab: 'downloader', icon: 'download' },
    { tab: 'packs', icon: 'package' },
    { tab: 'trainer', icon: 'sliders-horizontal' },
    { tab: 'replays', icon: 'activity' },
  ].map(({ tab, icon }) => ({ label: `settings.tabs.${tab}`, icon, translate: true }));
  /** Being saved, as it closes. */
  readonly saving = signal(false);
  readonly tools = USER_ITEMS.filter(item => item.route !== '/');

  /**
   * Local copy edited by the form; saved as the drawer closes. Copied again
   * from the saved settings each time it opens; but not when those change
   * while it is open. The osu! folder is the one in use, empty without one:
   * it may have been detected or chosen after the settings were read.
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

  readonly skillDisplays = ['dan', 'msd', 'both', 'none'] as const;
  readonly skillDisplayOptions: OptionGroupItem[] = this.skillDisplays.map(display => ({
    label: `settings.trainer.skillDisplay.${display}`,
    translate: true,
  }));
  readonly skillDisplayIndex = computed(() =>
    Math.max(0, this.skillDisplays.indexOf(this.draft().trainer.skill_display ?? 'dan')),
  );

  setSkillDisplay(index: number) {
    this.draft.update(draft => ({ ...draft, trainer: { ...draft.trainer, skill_display: this.skillDisplays[index] } }));
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

  /**
   * Saves what changed and closes; there is no cancelling. If the osu!
   * folder given is not osu!'s it stays open, saying so, to correct it.
   */
  async close() {
    if (this.saving()) return;
    this.saving.set(true);
    try {
      if (await this.save()) this.open.set(false);
    } finally {
      this.saving.set(false);
    }
  }

  /** Whether it was saved (or nothing had changed). */
  private async save(): Promise<boolean> {
    const draft = this.draft();
    const path = draft.osu_path;

    // A new osu! folder is checked first: nothing is saved if it is not osu!'s.
    // Emptied, it is forgotten and the app asks for one again.
    const pathChanged = (path ?? null) !== this.osuPath.path();
    if (pathChanged) {
      try {
        if (path) await this.osuPath.setPath(path);
        else await this.osuPath.clearPath();
      } catch (e) {
        this.notify.error(errorText(this.transloco, e));
        return false;
      }
    }

    const saved = this.store.config() ?? DEFAULT_CONFIG;
    const settingsChanged = comparable(draft) !== comparable(saved);
    if (!settingsChanged && !pathChanged) return true;

    if (!(await this.store.updateConfig(draft))) {
      this.notify.error(this.transloco.translate('settings.saveFailed'));
      return false;
    }
    this.notify.success(this.transloco.translate('settings.saved'));

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
    return true;
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
