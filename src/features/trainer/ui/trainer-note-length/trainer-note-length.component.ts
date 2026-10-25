import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { NoteLength } from '@features/trainer/models/trainer.model';
import { TuiInput } from '@taiga-ui/core';
import { TuiSegmented } from '@taiga-ui/kit';

const BEAT_DIVISORS = [1, 2, 3, 4, 6, 8, 12, 16];

/** Picks a length as a fraction of a beat or in milliseconds. */
@Component({
  selector: 'app-trainer-note-length',
  imports: [TuiInput, TuiSegmented],
  templateUrl: './trainer-note-length.component.html',
  styleUrl: './trainer-note-length.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerNoteLengthComponent {
  readonly label = input.required<string>();
  readonly hint = input.required<string>();
  readonly value = input.required<NoteLength>();
  /** BPM of the map, to show beats in ms and to convert between them. */
  readonly bpm = input<number | null>(null);
  readonly valueChange = output<NoteLength>();

  readonly divisors = BEAT_DIVISORS;

  readonly activeIndex = computed(() => {
    const value = this.value();
    return value.unit === 'ms' ? BEAT_DIVISORS.length : BEAT_DIVISORS.indexOf(value.value);
  });

  readonly approxMs = computed(() => {
    const bpm = this.bpm();
    const value = this.value();
    return bpm && value.unit === 'beat' ? Math.round(60000 / bpm / value.value) : null;
  });

  onSegment(index: number) {
    if (index < BEAT_DIVISORS.length) {
      this.valueChange.emit({ unit: 'beat', value: BEAT_DIVISORS[index] });
    } else if (this.value().unit !== 'ms') {
      // Starts from what the beat fraction was worth, so switching keeps it.
      this.valueChange.emit({ unit: 'ms', value: this.approxMs() ?? 50 });
    }
  }

  onMsInput(msStr: string) {
    const ms = parseFloat(msStr);
    if (isNaN(ms) || ms < 0) return;
    this.valueChange.emit({ unit: 'ms', value: Math.round(ms) });
  }
}
