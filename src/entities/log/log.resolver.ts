import { loaderResolver } from '@shared/stores';
import { LogStore } from './log.store';

export const logResolver = loaderResolver(LogStore, 'loading.logs');
