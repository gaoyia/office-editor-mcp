# 改过的工作表仍带着注释和处理指令。Excel 能打开，不报修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Read-ZipText([string]$zipPath, [string]$entryName) {
    $zip = [System.IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $entry = $zip.GetEntry($entryName)
        $stream = $entry.Open()
        try {
            $reader = New-Object System.IO.StreamReader($stream, [System.Text.UTF8Encoding]::new($false))
            try { return $reader.ReadToEnd() } finally { $reader.Dispose() }
        } finally { $stream.Dispose() }
    } finally { $zip.Dispose() }
}

function Write-ZipText([string]$zipPath, [string]$entryName, [string]$text) {
    $zip = [System.IO.Compression.ZipFile]::Open($zipPath, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $old = $zip.GetEntry($entryName)
        $old.Delete()
        $created = $zip.CreateEntry($entryName)
        $stream = $created.Open()
        try {
            $writer = New-Object System.IO.StreamWriter($stream, [System.Text.UTF8Encoding]::new($false))
            try { $writer.Write($text) } finally { $writer.Dispose() }
        } finally { $stream.Dispose() }
    } finally { $zip.Dispose() }
}

$src = Get-OutPath 'xlsx-xml-comment-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xml-comment-create' | Out-Null

$sheet = 'xl/worksheets/sheet1.xml'
$xml = Read-ZipText $src $sheet
$xml = $xml.Replace('<sheetData>', "<!--keep--><?mso-info yes?><sheetData>")
Write-ZipText $src $sheet $xml

$out = Get-OutPath 'xlsx-xml-comment.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(@{ op = 'setCell'; id = 'cell:Sheet1!B1'; value = 2 })
} 'xlsx-xml-comment-apply' | Out-Null

$written = Read-ZipText $out $sheet
if (-not $written.Contains('<!--keep-->')) { throw '注释丢了' }
if (-not $written.Contains('<?mso-info yes?>')) { throw '处理指令丢了' }

$excel = Ensure-MsExcel $out
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $out
$cap = [string]$wb.Windows.Item(1).Caption
$ok = $cap -notmatch 'Repair'
Add-Check 'xlsx-xml-comment' $ok "caption=$cap"
Close-AllWorkbooks $excel
