import { NavItem } from './nav-item.model';
import { faHouse, faDownload, faFileExport,
    faFileImport, faFileZipper, faSliders,
    faFileLines, faDatabase, faNetworkWired,
    faClockRotateLeft,
    faHeart,
    faBug
} from '@fortawesome/free-solid-svg-icons';
import { faGithub } from '@fortawesome/free-brands-svg-icons';

export const USER_ITEMS: NavItem[] = [
    { label: 'Home', route: '/', icon: faHouse },
    { label: 'Downloader', route: '/user/downloader', icon: faDownload },
    { label: 'Export Collections', route: '/user/export-collections', icon: faFileExport },
    { label: 'Import Collections', route: '/user/import-collections', icon: faFileImport },
    { label: 'Export Song Folder', route: '/user/export-song-folder', icon: faFileExport },
    { label: 'Create Packs', route: '/user/create-packs', icon: faFileZipper },
    { label: 'Trainer', route: '/user/trainer', icon: faSliders }
];

export const DEV_ITEMS: NavItem[] = [
    { label: 'logs', route: '/dev/logs', icon: faFileLines },
    { label: 'database', route: '/dev/database', icon: faDatabase },
    { label: 'tosu-data', route: '/dev/tosu-data', icon: faNetworkWired }
];

export const APP_ITEMS: NavItem[] = [
    { label: 'Changelog', route: '/app/changelog', icon: faClockRotateLeft },
    { label: 'Github', route: '/app/github', icon: faGithub },
    { label: 'Issues', route: '/app/issues', icon: faBug },
    { label: 'Support', route: '/app/support', icon: faHeart }
]
    
