export interface DownloadItem {
    beatmapSetId: number;
    status: DownloadStatus;
    progress: number;
}

export type DownloadStatus = 'pending' | 'downloading' | 'done' | 'failed';