import { DecimalPipe } from '@angular/common';
import { Component, inject } from '@angular/core';
import { TosuStore } from '@entities/tosu';

@Component({
  selector: 'app-trainer-beatmap-header',
  imports: [DecimalPipe],
  templateUrl: './beatmap-header.component.html',
  styleUrl: './beatmap-header.component.css',
})
export class TrainerBeatmapHeaderComponent {

  readonly tosu = inject(TosuStore);

  
}
