import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { ButtonComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ConfigurationStore } from '@entities/configuration';
import { ConfigDrawerComponent } from '../config-drawer/config-drawer.component';

/**
 * The settings button. The drawer and its form (Taiga kit, maskito, forms)
 * are only downloaded the first time it opens.
 */
@Component({
  selector: 'app-config-button',
  imports: [ButtonComponent, ConfigDrawerComponent, TranslocoPipe],
  template: `
    <button appButton="ghost" icon="settings" iconOnly type="button" (click)="toggle()">
      {{ 'header.settings' | transloco }}
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
