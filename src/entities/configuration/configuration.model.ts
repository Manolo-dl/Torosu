export interface AppConfig {
    osu_path?: string;
    excluded_tools: string[];
    tosu: TosuConfig;
    downloader: DownloaderConfig;
    sqlite_db: boolean;
    open_file: boolean;
}

export interface TosuConfig {
    auto_start: boolean;
    kill_on_exit: boolean; // Kill the process only when started by the app, not if it was already running
    ip: string;
    port: number;
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