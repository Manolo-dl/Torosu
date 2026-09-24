import { NgClass } from '@angular/common';
import { Component, inject } from '@angular/core';
import { ToastStore } from '@shared/stores';

const TOAST_ICON: Record<string, string> = {
  success: 'phosphorCheckCircle',
  error:   'phosphorXCircle',
  warning: 'phosphorWarningCircle',
  info:    'phosphorInfo',
};

@Component({
  selector: 'app-toast',
  imports: [NgClass],
  templateUrl: './toast.component.html',
  styleUrl: './toast.component.css',
})
export class ToastComponent {
  readonly store = inject(ToastStore);
  readonly toastIcon = TOAST_ICON;
}
