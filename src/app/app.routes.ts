import { Routes } from "@angular/router";
import { collectionResolver } from "@entities/collection";
import { osuDbResolver } from "@entities/osu-db";
import { logResolver } from "@entities/log";
import { osuPathGuard } from "@shared/guards";
import { databaseResolver } from "@entities/database";
import { osuReaderResolver } from "@entities/osu-reader";
import { devModeGuard, toolEnabledGuard } from "@entities/configuration";

export const routes: Routes = [
    {
        path: '',
        loadComponent: () => import('@pages/home').then(m => m.HomePageComponent)
    },
    {
        path: 'user',
        canActivate: [osuPathGuard],
        canActivateChild: [toolEnabledGuard],
        children: [
            {
                path: 'downloader',
                loadComponent: () => import('@pages/downloader').then(m => m.DownloaderPageComponent)
            },
            {
                path: 'export-collections',
                loadComponent: () => import('@pages/export-collections').then(m => m.ExportCollectionsPageComponent),
                resolve: { osuDb: osuDbResolver, collections: collectionResolver }
            },
            {
                path: 'export-song-folder',
                loadComponent: () => import('@pages/export-song-folder').then(m => m.ExportSongFolderPageComponent),
                resolve: { osuDb: osuDbResolver }
            },
            {
                path: 'create-packs',
                loadComponent: () => import('@pages/create-packs').then(m => m.CreatePacksPageComponent),
                resolve: { osuDb: osuDbResolver }
            },
            {
                path: 'import-collections',
                loadComponent: () => import('@pages/import-collections').then(m => m.ImportCollectionsPageComponent),
                resolve: { osuDb: osuDbResolver, collections: collectionResolver }
            },
            {
                path: 'trainer',
                loadComponent: () => import('@pages/trainer').then(m => m.TrainerPageComponent),
                resolve: { osuReader: osuReaderResolver }
            }
        ]
    },
    {
        path: 'dev',
        canActivate: [devModeGuard],
        children: [
            {
                path: 'logs',
                loadComponent: () => import('@pages/logs').then(m => m.LogsPageComponent),
                resolve: { logs: logResolver }
            },
            {
                path: 'database',
                loadComponent: () => import('@pages/database').then(m => m.DatabasePageComponent),
                resolve: { tables: databaseResolver }
            },
            {
                path: 'osu-data',
                loadComponent: () => import('@pages/osu-data').then(m => m.OsuDataPage),
                resolve: { osuReader: osuReaderResolver }
            }
        ]
    },
    {
        path: '**',
        redirectTo: '',
    }
];
