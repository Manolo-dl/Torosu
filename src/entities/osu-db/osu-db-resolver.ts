import { loaderResolver } from '@shared/stores';
import { OsuDbStore } from './osu-db.store';

export const osuDbResolver = loaderResolver(OsuDbStore, 'loading.osuDb');
