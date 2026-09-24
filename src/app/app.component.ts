import { TUI_DARK_MODE, TuiRoot } from "@taiga-ui/core";
import { Component, inject, OnInit } from "@angular/core";
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

  ngOnInit() {
    this.darkMode.set(true);
    this.platformStore.setOs();
  }
}
