import { DecimalPipe } from '@angular/common';
import { Component, inject } from '@angular/core';
import { DifficultyKey } from '@features/trainer';
import { TrainerStore } from '@features/trainer';

@Component({
  selector: 'app-trainer-difficulty-sliders',
  imports: [DecimalPipe],
  templateUrl: './difficulty-sliders.component.html',
  styleUrl: './difficulty-sliders.component.css',
})
export class TrainerDifficultySlidersComponent {
  
  readonly trainer = inject(TrainerStore);

  onSliderChange(stat: DifficultyKey, event: Event) {
    const value = Number((event.target as HTMLInputElement).value);
    this.trainer.setValue(stat, value);
  }
}
