import { TUI_DARK_MODE, TuiRoot } from "@taiga-ui/core";
import { Component, DOCUMENT, effect, inject, OnInit } from "@angular/core";
import { RouterOutlet } from "@angular/router";
import { HeaderComponent } from "@widgets/header";
import { SidebarComponent } from "@widgets/sidebar";
import { OsuPathComponent } from "@widgets/osu-path";
import { ToastComponent } from "@widgets/toast";
import { PlatformStore } from "@shared/stores";
import { WindowControlsComponent } from "@widgets/window-controls/window-controls.component";

@Component({
  selector: "app-root",
  imports: [RouterOutlet, HeaderComponent, SidebarComponent, OsuPathComponent, ToastComponent, TuiRoot, WindowControlsComponent],
  templateUrl: "./app.component.html",
  styleUrl: "./app.component.less",
})
export class AppComponent implements OnInit {

  readonly darkMode = inject(TUI_DARK_MODE);
  readonly platformStore = inject(PlatformStore);
  private readonly document = inject(DOCUMENT);

  constructor() {
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
  }
}
