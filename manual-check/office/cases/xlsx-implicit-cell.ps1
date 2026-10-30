# 三行格子都不写 r。inspect 能读到；setCell 改中 B2、追加 A5。
# Excel 打开不报修复，B2 是 50，A5 是 70。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Read-ZipText([string]$zipPath, [string]$entryName) {
    $zip = [System.IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $entry = $zip.GetEntry($entryName)
        if (-not $entry) { throw "找不到 $entryName" }
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
        if (-not $old) { throw "找不到 $entryName" }
        $old.Delete()
        $created = $zip.CreateEntry($entryName)
        $stream = $created.Open()
        try {
            $writer = New-Object System.IO.StreamWriter($stream, [System.Text.UTF8Encoding]::new($false))
            try { $writer.Write($text) } finally { $writer.Dispose() }
        } finally { $stream.Dispose() }
    } finally { $zip.Dispose() }
}

$cells = @()
$n = 1
foreach ($row in 1..3) {
    foreach ($col in @('A', 'B', 'C')) {
        $cells += @{ ref = "$col$row"; value = $n }
        $n++
    }
}

$src = Get-OutPath 'xlsx-implicit-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-implicit-create' | Out-Null

$sheet = 'xl/worksheets/sheet1.xml'
$xml = Read-ZipText $src $sheet
$xml = $xml -replace '<row r="\d+"', '<row'
$xml = $xml -replace '<c r="[A-Z]+\d+"', '<c'
if ($xml -match '<c r=') { throw '格子上的 r 没去掉' }
Write-ZipText $src $sheet $xml

$out = Get-OutPath 'xlsx-implicit.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{ op = 'setCell'; id = 'cell:Sheet1!B2'; value = 50 }
        @{ op = 'setCell'; id = 'cell:Sheet1!A5'; value = 70 }
    )
} 'xlsx-implicit-apply' | Out-Null

$written = Read-ZipText $out $sheet
if ($written -notmatch '<c r="B2"><v>50</v></c>') { throw "B2 没写成 50：$written" }
if ($written -notmatch '<c r="A5"><v>70</v></c>') { throw "A5 没写成 70：$written" }

$excel = Ensure-MsExcel $out
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $out
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item('Sheet1')
$b2 = $ws.Range('B2').Value2
$a5 = $ws.Range('A5').Value2
$a1 = $ws.Range('A1').Value2
$ok = ($cap -notmatch 'Repair') -and ([double]$b2 -eq 50) -and ([double]$a5 -eq 70) -and ([double]$a1 -eq 1)
Add-Check 'xlsx-implicit-cell' $ok "caption=$cap A1=$a1 B2=$b2 A5=$a5"
Release-Com $ws
Close-AllWorkbooks $excel
