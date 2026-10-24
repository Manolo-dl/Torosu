export interface AppConfig {
    osu_path?: string;
    excluded_tools: string[];
    downloader: DownloaderConfig;
    sqlite_db: boolean;
    open_file: boolean;
}

export interface DownloaderConfig {
    notify: boolean;
    error_file: boolean;
    osu_session?: string;
    output_path?: string;
    load_library: boolean;
    skip_video: boolean;
    max_concurrent_downloads: number;
}