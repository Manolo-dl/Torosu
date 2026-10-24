export interface SelectedDiff {
    md5: string,
    folderName: string,
    fileName: string,
    audio: string
    newDiffName: string
}

export interface PackRequest {
    title: string,
    finalCreator: string,
    diffs: SelectedDiff[]
}