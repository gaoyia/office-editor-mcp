# Item-level ownership tracking. Dot-source office-com.ps1 first. This does not
# start, rebind, Quit, kill or close an application window. In particular an
# empty Excel process is NOT evidence that application/window cleanup succeeded.
function New-OwnedExcelBooks($App, [string]$EvidenceRoot, [string[]]$OwnedPaths) {
    $root = [IO.Path]::GetFullPath($EvidenceRoot).TrimEnd('\') + '\'
    $temp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if (-not $root.StartsWith($temp, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Evidence root must be an item-owned Temp directory'
    }
    $allowed = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($path in $OwnedPaths) {
        $full = [IO.Path]::GetFullPath($path)
        if (-not $full.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) {
            throw 'Owned workbook must be explicitly named within the evidence root'
        }
        [void]$allowed.Add($full)
    }
    [uint32]$ownerPid = 0
    [void][OoxpOfficeOm]::GetWindowThreadProcessId([IntPtr]$App.Hwnd, [ref]$ownerPid)
    if (-not $ownerPid) { throw 'Cannot establish Excel process identity' }
    [pscustomobject]@{ App=$App; ProcessId=$ownerPid; Allowed=$allowed
        StartTime=(Get-Process -Id $ownerPid).StartTime; Books=[Collections.Generic.List[object]]::new() }
}
function Add-OwnedExcelBook($Scope, $Book) {
    $full = [IO.Path]::GetFullPath([string]$Book.FullName)
    if (-not $Scope.Allowed.Contains($full)) { throw "Not an explicitly owned workbook: $full" }
    if (@($Scope.Books | Where-Object { $_.Path -ieq $full }).Count) {
        throw "Workbook already tracked: $full"
    }
    $bookApp=$null
    try {
        $bookApp=$Book.Application
        [uint32]$bookPid=0
        [void][OoxpOfficeOm]::GetWindowThreadProcessId([IntPtr]$bookApp.Hwnd,[ref]$bookPid)
        if ($bookPid -ne $Scope.ProcessId) { throw 'Workbook belongs to another Excel process' }
    } finally { Release-Com $bookApp }
    # Caller transfers its explicitly held book reference to this tracker.
    $Scope.Books.Add([pscustomobject]@{ Path=$full; Book=$Book; Closed=$false; Error=$null })
}
function Close-OwnedExcelBooks($Scope) {
    [uint32]$currentPid = 0
    [void][OoxpOfficeOm]::GetWindowThreadProcessId([IntPtr]$Scope.App.Hwnd, [ref]$currentPid)
    if ($currentPid -ne $Scope.ProcessId -or (Get-Process -Id $currentPid).StartTime -ne $Scope.StartTime) { throw 'Excel identity changed; refuse cleanup' }
    for ($i=$Scope.Books.Count-1; $i -ge 0; $i--) {
        $record=$Scope.Books[$i]
        try {
            if ([string]$record.Book.FullName -ine $record.Path) { throw 'Workbook identity changed' }
            $record.Book.Close($false)
            $record.Closed=$true
        } catch { $record.Error=$_.Exception.Message }
        finally { Release-Com $record.Book; $record.Book=$null }
    }
    $books=$null; $remaining=@(); $verificationError=$null
    try {
        $books=$Scope.App.Workbooks
        for ($i=1; $i -le $books.Count; $i++) {
            $book=$null
            try { $book=$books.Item($i); $remaining += [string]$book.FullName }
            finally { Release-Com $book }
        }
    } catch { $verificationError=$_.Exception.Message }
    finally { Release-Com $books }
    foreach ($record in $Scope.Books) {
        if ($remaining -contains $record.Path) { $record.Closed=$false; $record.Error='Still present after Close(false)' }
        if ($verificationError) { $record.Closed=$false; $record.Error='Close verification failed: '+$verificationError }
    }
    $proc=Get-Process -Id $Scope.ProcessId -ErrorAction SilentlyContinue
    [pscustomobject]@{ Books=@($Scope.Books.ToArray()); RemainingWorkbookCount=$remaining.Count
        VerificationError=$verificationError; ProcessStillRunning=[bool]$proc
        WindowStillPresent=([bool]$proc -and $proc.MainWindowHandle -ne [IntPtr]::Zero) }
}
# Call Close-OwnedExcelBooks in finally, including export failures. Release chart,
# range, window and collection references first, then the application reference.
# Register the seed opened by Ensure-MsExcel as well as the read-only export copy.
# If WindowStillPresent is true, report it. Do not launch another /x session to
# hide/pile up blank windows; never close an unrelated/default/user workbook.