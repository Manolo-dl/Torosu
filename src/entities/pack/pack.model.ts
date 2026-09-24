export interface SelectedDiff {
    md5: string,
    beatmapsetId: number,
    fileName: string,
    audio: string
    newDiffName: string
}

export interface PackRequest {
    title: string,
    finalCreator: string,
    diffs: SelectedDiff[]
}