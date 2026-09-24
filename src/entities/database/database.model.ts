export interface DatabaseTable {
    name: string;
    columns: DatabaseColumn[];
}

export interface DatabaseColumn {
    cid: number;
    name: string;
    columnType: string;
    notnull: boolean;
    dfltValue: string | null;
    pk: boolean
}