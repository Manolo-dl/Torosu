export interface DownloadItem {
    beatmapSetId: number;
    status: DownloadStatus;
    progress: number;
    /** "Artist - Title", once its download has started and osu! has said it. */
    name?: string;
}

export type DownloadStatus = 'pending' | 'downloading' | 'done' | 'failed';