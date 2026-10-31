import { NavItem } from './nav-item.model';

export const USER_ITEMS: NavItem[] = [
    { label: 'Home', route: '/', icon: '@tui.house' },
    { label: 'Downloader', route: '/user/downloader', icon: '@tui.download' },
    { label: 'Export Collections', route: '/user/export-collections', icon: '@tui.file-output' },
    { label: 'Import Collections', route: '/user/import-collections', icon: '@tui.file-input' },
    { label: 'Export Song Folder', route: '/user/export-song-folder', icon: '@tui.folder-output' },
    { label: 'Create Packs', route: '/user/create-packs', icon: '@tui.package' },
    { label: 'Trainer', route: '/user/trainer', icon: '@tui.sliders-horizontal' }
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
    
