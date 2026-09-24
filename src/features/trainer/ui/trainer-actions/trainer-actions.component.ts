import { Component, inject } from '@angular/core';
import { TrainerStore } from '@features/trainer/stores/trainer.store';

@Component({
  selector: 'app-trainer-actions',
  imports: [],
  templateUrl: './trainer-actions.component.html',
  styleUrl: './trainer-actions.component.css',
})
export class TrainerActionsComponent {
  readonly trainer = inject(TrainerStore);
}
