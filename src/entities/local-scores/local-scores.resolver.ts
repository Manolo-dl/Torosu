import { loaderResolver } from '@shared/stores';
import { LocalScoresStore } from './local-scores.store';

export const localScoresResolver = loaderResolver(LocalScoresStore);
