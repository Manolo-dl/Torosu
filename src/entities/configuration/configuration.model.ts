export interface AppConfig {
    osu_path?: string;
    excluded_tools: string[];
    downloader: DownloaderConfig;
    packs: PacksConfig;
    trainer: TrainerConfig;
    open_file: boolean;
    /** Shows the developer pages (logs, osu! data). */
    dev_mode: boolean;
}

export interface TrainerConfig {
    /** Encode the generated audio at ~192 kbit/s instead of ~128 kbit/s. */
    high_quality_audio: boolean;
    /** Re-encode the scaled video at a better quality (CRF 23 instead of 28), nearly twice as big. */
    high_quality_video: boolean;
    /** The rate change sets the pitch from the rate (kept below 1.0x, raised above), without the option. */
    auto_pitch: boolean;
    /** Open each generated .osz, so osu! imports it right away. */
    open_osz: boolean;
    /** Template of the difficulty name, with placeholders like `{diff}` and `{rate}` (see `naming.rs`). */
    name_format: string;
}

export interface PacksConfig {
    /** Copy each diff's background into the pack. */
    keep_background: boolean;
    /** Above this many original sets (each with its own audio and background) the pack is split; 0 never splits. */
    max_sets_per_pack: number;
}

export interface DownloaderConfig {
    notify: boolean;
    error_file: boolean;
    /** The saved cookie; set from the downloader, and only kept with `save_session`. */
    osu_session?: string;
    /** Keep the osu_session cookie between sessions; off, it's forgotten when Torosu closes. */
    save_session: boolean;
    output_path?: string;
    load_library: boolean;
    skip_video: boolean;
    max_concurrent_downloads: number;
}