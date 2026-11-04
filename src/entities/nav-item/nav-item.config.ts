import { NavItem } from './nav-item.model';

export const USER_ITEMS: NavItem[] = [
    { label: 'nav.home', route: '/', icon: '@tui.house' },
    {
        label: 'nav.downloader', route: '/user/downloader', icon: '@tui.download',
        description: 'nav.downloaderDescription',
    },
    {
        label: 'nav.exportCollections', route: '/user/export-collections', icon: '@tui.file-output',
        description: 'nav.exportCollectionsDescription',
    },
    {
        label: 'nav.importCollections', route: '/user/import-collections', icon: '@tui.file-input',
        description: 'nav.importCollectionsDescription',
    },
    {
        label: 'nav.exportSongFolder', route: '/user/export-song-folder', icon: '@tui.folder-output',
        description: 'nav.exportSongFolderDescription',
    },
    {
        label: 'nav.createPacks', route: '/user/create-packs', icon: '@tui.package',
        description: 'nav.createPacksDescription',
    },
    {
        label: 'nav.trainer', route: '/user/trainer', icon: '@tui.sliders-horizontal',
        description: 'nav.trainerDescription',
    },
];

export const DEV_ITEMS: NavItem[] = [
    { label: 'nav.logs', route: '/dev/logs', icon: '@tui.file-text' },
    { label: 'nav.osuData', route: '/dev/osu-data', icon: '@tui.network' }
];

export const APP_ITEMS: NavItem[] = [
    { label: 'nav.changelog', route: '/app/changelog', icon: '@tui.history' },
    { label: 'nav.github', route: '/app/github', icon: '@tui.github' },
    { label: 'nav.issues', route: '/app/issues', icon: '@tui.bug' },
    { label: 'nav.support', route: '/app/support', icon: '@tui.heart' }
]
    
