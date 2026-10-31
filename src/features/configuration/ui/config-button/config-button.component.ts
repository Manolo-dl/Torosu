import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { TuiButton } from '@taiga-ui/core';
import { ConfigurationStore } from '@entities/configuration';
import { ConfigDrawerComponent } from '../config-drawer/config-drawer.component';

/**
 * The settings button. The drawer and its form (Taiga kit, maskito, forms)
 * are only downloaded the first time it opens.
 */
@Component({
  selector: 'app-config-button',
  imports: [TuiButton, ConfigDrawerComponent],
  template: `
    <button
      size="m"
      appearance="flat"
      iconStart="@tui.settings"
      tuiIconButton
      type="button"
      (click)="toggle()"
    >
      Settings
    </button>

    @defer (when open()) {
      <app-config-drawer [(open)]="open" />
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ConfigButtonComponent {
  private readonly store = inject(ConfigurationStore);

  readonly open = signal(false);

  async toggle() {
    if (this.open()) {
      this.open.set(false);
      return;
    }

    // The drawer starts from the saved settings.
    await this.store.ensureLoaded();
    this.open.set(true);
  }
}
