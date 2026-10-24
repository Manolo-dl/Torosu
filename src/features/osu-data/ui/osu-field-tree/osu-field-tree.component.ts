import { Component, input, ChangeDetectionStrategy } from '@angular/core';

@Component({
    selector: 'app-osu-field-tree',
    imports: [],
    changeDetection: ChangeDetectionStrategy.OnPush,
    templateUrl: './osu-field-tree.component.html',
    styleUrl: './osu-field-tree.component.less',
})
export class OsuFieldTreeComponent {
    value = input<unknown>();

    isObject(v: unknown): boolean {
        return v !== null && typeof v === 'object' && !Array.isArray(v);
    }
    isArray(v: unknown): boolean {
        return Array.isArray(v);
    }
    asArray(v: unknown): unknown[] {
        return v as unknown[];
    }
    entries(v: unknown): [string, unknown][] {
        return Object.entries(v as Record<string, unknown>);
    }
    typeLabel(v: unknown): string {
        return Array.isArray(v) ? `[${(v as unknown[]).length}]` : '{…}';
    }
    formatValue(v: unknown): string {
        if (v === null) return 'null';
        if (v === undefined) return 'undefined';
        if (typeof v === 'string') return `"${v}"`;
        return String(v);
    }
    valueType(v: unknown): string {
        if (v === null || v === undefined) return 'null';
        if (typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean') return typeof v;
        return 'null';
    }
}
