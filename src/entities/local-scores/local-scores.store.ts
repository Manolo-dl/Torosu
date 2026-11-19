import { computed } from '@angular/core';
import { signalStore, withComputed } from '@ngrx/signals';
import { withLoader } from '@shared/stores';
import { invoke } from '@tauri-apps/api/core';
import { LocalScore } from './local-scores.model';

interface LocalScoresState {
    /** Newest first. */
    scores: LocalScore[];
}

const initialState: LocalScoresState = {
    scores: [],
};

/** The mania plays osu! keeps on this computer (`scores.db`). */
export const LocalScoresStore = signalStore(
    { providedIn: 'root' },
    withLoader(initialState, {
        read: () => invoke<LocalScore[]>('list_local_scores'),
        loaded: scores => ({ scores }),
        failure: 'loading.localScoresFailed',
    }),

    withComputed(store => ({
        /** Only those osu! still has the replay of, which can be analysed. */
        withReplay: computed(() => store.scores().filter(score => score.replay !== null)),
    })),
);
