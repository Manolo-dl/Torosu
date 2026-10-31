import { loaderResolver } from '@shared/stores';
import { LogStore } from './log.store';

export const logResolver = loaderResolver(LogStore, 'Loading logs, please wait...');
