import { computed } from '@angular/core';
import { patchState, signalStore, withComputed, withHooks, withMethods } from '@ngrx/signals';
import { withLoader } from '@shared/stores';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { LocalScore } from './local-scores.model';

interface LocalScoresState {
    /** Newest first. */
    scores: LocalScore[];
    /** The last play osu! saved while Torosu runs (`local-score-added`). */
    added: LocalScore | null;
}

const initialState: LocalScoresState = {
    scores: [],
    added: null,
};

/** The mania plays osu! keeps on this computer (`scores.db` and `Data/r`). */
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

    withMethods(store => ({
        /** A play osu! just saved, first in the list unless it is there already. */
        add(score: LocalScore) {
            const known = store.scores().some(s => s.replay !== null && s.replay === score.replay);
            patchState(store, { scores: known ? store.scores() : [score, ...store.scores()], added: score });
        },
    })),

    withHooks({
        onInit(store) {
            // The osu! reader sends them while it runs (see `osuReaderResolver`).
            listen<LocalScore>('local-score-added', event => store.add(event.payload)).catch(e =>
                console.error('could not listen to local-score-added', e),
            );
        },
    }),
);
