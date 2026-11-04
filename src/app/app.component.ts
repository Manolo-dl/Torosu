import { TUI_DARK_MODE, TuiRoot } from "@taiga-ui/core";
import { Component, DOCUMENT, effect, inject, OnInit, signal, untracked } from "@angular/core";
import { RouterOutlet } from "@angular/router";
import { HeaderComponent } from "@widgets/header";
import { SidebarComponent } from "@widgets/sidebar";
import { OsuPathComponent } from "@widgets/osu-path";
import { OsuPathStore } from "@entities/osu-path";
import { PlatformStore } from "@shared/stores";
import { ConfigurationStore } from "@entities/configuration";
import { WindowControlsComponent } from "@widgets/window-controls/window-controls.component";
import { TrainerShortcutsService } from "@features/trainer";
import { isLanguage, LanguageService, systemLanguage } from "@shared/i18n";

@Component({
  selector: "app-root",
  imports: [RouterOutlet, HeaderComponent, SidebarComponent, OsuPathComponent, TuiRoot, WindowControlsComponent],
  templateUrl: "./app.component.html",
  styleUrl: "./app.component.less",
})
export class AppComponent implements OnInit {

  readonly darkMode = inject(TUI_DARK_MODE);
  readonly platformStore = inject(PlatformStore);
  private readonly config = inject(ConfigurationStore);
  readonly osuPath = inject(OsuPathStore);
  private readonly document = inject(DOCUMENT);
  // Registers the trainer's keyboard shortcuts as the settings say, from the
  // start: they work while osu! has the focus, whatever page is open.
  private readonly trainerShortcuts = inject(TrainerShortcutsService);
  private readonly language = inject(LanguageService);
  /** The settings were read (or failed to be): the language to use is known. */
  private readonly settingsRead = signal(false);

  constructor() {
    // The language saved in the settings, or the system's; again when it
    // changes. Not before they are read, or the system's would show first.
    effect(() => {
      if (!this.settingsRead()) return;
      const chosen = this.config.config()?.language;
      untracked(async () => this.language.use(isLanguage(chosen) ? chosen : await systemLanguage()));
    });

    // Taiga surfaces animate their background, so without this they fade
    // to the new theme after everything else has already switched.
    effect(() => {
      this.darkMode();
      const root = this.document.documentElement;
      root.classList.add('theme-switching');
      requestAnimationFrame(() => requestAnimationFrame(() => root.classList.remove('theme-switching')));
    });
  }

  ngOnInit() {
    this.darkMode.set(true);
    this.platformStore.setOs();
    // The sidebar and the tools read it (e.g. which tools are turned off).
    this.config.load().finally(() => this.settingsRead.set(true));
    // Without a folder the widget above the pages asks for one.
    this.osuPath.detect();
  }
}
