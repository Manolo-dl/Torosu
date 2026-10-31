import { NavItem } from './nav-item.model';

export const USER_ITEMS: NavItem[] = [
    { label: 'Home', route: '/', icon: '@tui.house' },
    {
        label: 'Downloader', route: '/user/downloader', icon: '@tui.download',
        description: 'Download beatmap sets from a list of links straight into your Songs folder, skipping the ones you have.',
    },
    {
        label: 'Export Collections', route: '/user/export-collections', icon: '@tui.file-output',
        description: 'Save your collections as beatmap links, or as a list to import on another install.',
    },
    {
        label: 'Import Collections', route: '/user/import-collections', icon: '@tui.file-input',
        description: 'Bring collections from a file into osu!, merged with the ones you already have.',
    },
    {
        label: 'Export Song Folder', route: '/user/export-song-folder', icon: '@tui.folder-output',
        description: 'Filter your library by mode, status, stars, length and more, and export it as beatmap links.',
    },
    {
        label: 'Create Packs', route: '/user/create-packs', icon: '@tui.package',
        description: 'Put diffs from any sets, rates and unsubmitted maps too, into a pack of your own.',
    },
    {
        label: 'Trainer', route: '/user/trainer', icon: '@tui.sliders-horizontal',
        description: 'Change the rate, the stats, the long notes or the key count of the map selected in osu!.',
    },
];

export const DEV_ITEMS: NavItem[] = [
    { label: 'logs', route: '/dev/logs', icon: '@tui.file-text' },
    { label: 'osu-data', route: '/dev/osu-data', icon: '@tui.network' }
];

export const APP_ITEMS: NavItem[] = [
    { label: 'Changelog', route: '/app/changelog', icon: '@tui.history' },
    { label: 'Github', route: '/app/github', icon: '@tui.github' },
    { label: 'Issues', route: '/app/issues', icon: '@tui.bug' },
    { label: 'Support', route: '/app/support', icon: '@tui.heart' }
]
    
