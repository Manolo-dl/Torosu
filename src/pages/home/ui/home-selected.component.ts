import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { RouterLink } from '@angular/router';
import { TranslocoPipe } from '@jsverse/transloco';
import { OsuReaderStore } from '@entities/osu-reader';
import { ButtonComponent, SectionTitleComponent, StarBadgeComponent } from '@shared/ui';

/**
 * The map selected in osu!, over its background: shown once the osu! reader
 * is connected (the trainer or the replay analysis start it); the home does
 * not start it.
 */
@Component({
    selector: 'app-home-selected',
    imports: [ButtonComponent, RouterLink, SectionTitleComponent, StarBadgeComponent, TranslocoPipe],
    template: `
        @if (reader.connected() && reader.data()) {
            <h2 appSectionTitle>{{ 'home.selected.title' | transloco }}</h2>
            <div class="selected">
                @if (reader.backgroundUrl(); as background) {
                    <img class="background" [src]="background" alt="" />
                }
                <div class="text">
                    <strong class="title">{{ reader.title() }}</strong>
                    <span class="artist">{{ reader.artist() }}</span>
                    <span class="meta">
                        <app-star-badge size="s" [stars]="reader.stars()" />
                        <span>{{ reader.version() }}</span>
                        <span class="by">{{ 'home.selected.by' | transloco: { mapper: reader.mapper() } }}</span>
                    </span>
                </div>
                <a appButton="secondary" size="s" icon="sliders-horizontal" routerLink="/user/trainer">
                    {{ 'home.selected.trainer' | transloco }}
                </a>
            </div>
        }
    `,
    styles: `
        :host {
            display: block;
        }

        :host:empty {
            display: none;
        }

        .selected {
            position: relative;
            display: flex;
            align-items: center;
            gap: var(--space-4);
            min-block-size: 6.5rem;
            padding: var(--space-4) var(--space-5);
            border: 1px solid var(--line-subtle);
            border-radius: var(--radius-lg);
            background: var(--surface-1);
            overflow: hidden;
            isolation: isolate;
            animation: rise-in var(--duration-4) var(--ease-out) backwards;
        }

        /* The background, fading out to the left where the text is. */
        .background {
            position: absolute;
            inset: 0;
            z-index: -1;
            inline-size: 100%;
            block-size: 100%;
            object-fit: cover;
            opacity: 0.45;
            -webkit-mask-image: linear-gradient(90deg, transparent 10%, #000 85%);
            mask-image: linear-gradient(90deg, transparent 10%, #000 85%);
            animation: fade-in var(--duration-4) var(--ease-out);
        }

        .text {
            display: flex;
            flex: 1;
            flex-direction: column;
            gap: 0.15rem;
            min-inline-size: 0;
        }

        .title {
            overflow: hidden;
            color: var(--text-1);
            font-family: var(--font-display);
            font-size: var(--text-lg);
            font-weight: var(--weight-black);
            text-overflow: ellipsis;
            white-space: nowrap;
        }

        .artist {
            color: var(--text-2);
            font-size: var(--text-sm);
        }

        .meta {
            display: flex;
            flex-wrap: wrap;
            align-items: center;
            gap: var(--space-2);
            margin-block-start: var(--space-1);
            color: var(--text-2);
            font-size: var(--text-xs);

            .by {
                color: var(--text-3);
            }
        }

        @keyframes rise-in {
            from { opacity: 0; transform: translateY(10px); }
        }

        @keyframes fade-in {
            from { opacity: 0; }
        }
    `,
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HomeSelectedComponent {
    readonly reader = inject(OsuReaderStore);
}
