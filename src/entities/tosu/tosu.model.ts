export interface TosuData {
    beatmap: {
        artist: string;
        artistUnicode: string;
        mapper: string;
        mode: { name: string; number: number };
        stats: {
            ar: { converted: number, original: number };
            bpm: { common: number, min: number, max: number, realtime: number };
            cs: { converted: number, original: number };
            hp: { converted: number, original: number };
            od: { converted: number, original: number };
            stars: {
                hitwindow: number;
                live: number;
                total: number;
            };
        },
        status: { name: string, number: number };
        time: {
            firstObject: number;
            lastObject: number;
            live: number;
            mp3Length: number;
        };
        title: string;
        titleUnicode: string;
        version: string;
    };

    client: string;

    directPath: {
        beatmapAudio: string;
        beatmapBackground: string;
        beatmapFile: string;
        beatmapFolder: string;
        skinFolder: string;
    };

    files: {
        audio: string;
        background: string;
        beatmap: string;
    };
    
    [key: string]: unknown;
}