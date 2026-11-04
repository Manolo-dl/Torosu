import { loaderResolver } from '@shared/stores';
import { CollectionStore } from './collection.store';

export const collectionResolver = loaderResolver(CollectionStore, 'loading.collections');
