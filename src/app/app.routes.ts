import { Routes } from "@angular/router";
import { collectionResolver } from "@entities/collection";
import { osuDbResolver } from "@entities/osu-db";
import { logResolver } from "@entities/log";
import { osuPathGuard } from "@shared/guards";
import { databaseResolver } from "@entities/database";
import { tosuResolver } from "@entities/tosu";

export const routes: Routes = [
    {
        path: '',
        loadComponent: () => import('@pages/home').then(m => m.HomePageComponent)
    },
    {
        path: 'user',
        canActivate: [osuPathGuard],
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
                resolve: { tosu: tosuResolver }
            }
        ]
    },
    {
        path: 'dev',
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
                path: 'tosu-data',
                loadComponent: () => import('@pages/tosu-data').then(m => m.TosuDataPage),
                resolve: { tosu: tosuResolver }
            }
        ]
    },
    {
        path: '**',
        redirectTo: '',
    }
];
