import { Injectable, signal } from '@angular/core';

export type ToastTone = 'success' | 'danger' | 'info';

export interface Toast {
  id: number;
  message: string;
  tone: ToastTone;
}

/** At most this many at once: a new one pushes the oldest out. */
const MAX_TOASTS = 4;

/**
 * The app's notifications, shown by `app-toasts` (in AppComponent) at the
 * bottom right, each for a few seconds unless the pointer rests on it.
 */
@Injectable({ providedIn: 'root' })
export class NotifyService {
  private nextId = 0;
  private readonly list = signal<readonly Toast[]>([]);
  readonly toasts = this.list.asReadonly();

  success(message: string) {
    this.show(message, 'success');
  }

  error(message: string) {
    this.show(message, 'danger');
  }

  info(message: string) {
    this.show(message, 'info');
  }

  dismiss(id: number) {
    this.list.update(toasts => toasts.filter(toast => toast.id !== id));
  }

  private show(message: string, tone: ToastTone) {
    const toast: Toast = { id: this.nextId++, message, tone };
    this.list.update(toasts => [...toasts, toast].slice(-MAX_TOASTS));
  }
}
