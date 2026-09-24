import { Component, input, ChangeDetectionStrategy } from '@angular/core';

@Component({
    selector: 'app-tosu-field-tree',
    imports: [],
    changeDetection: ChangeDetectionStrategy.OnPush,
    templateUrl: './tosu-field-tree.component.html',
    styleUrl: './tosu-field-tree.component.css',
})
export class TosuFieldTreeComponent {
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
    valueClass(v: unknown): string {
        if (v === null || v === undefined) return 'val-null';
        if (typeof v === 'string') return 'val-string';
        if (typeof v === 'number') return 'val-number';
        if (typeof v === 'boolean') return 'val-boolean';
        return 'val-null';
    }
}
