# pptx create 带图表：docs/checklist-pptx.md 的 C8–C10。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$path = Ensure-PptxCreateCharts
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-create-charts-inspect'

$charts = @($tree.slides[0].charts)
$c = $charts | Select-Object -First 1
Add-Check 'pptx-C8' (($null -ne $c) -and ($c.id -eq 'slide:1/chart:3') -and ($c.type -eq 'col')) "id=$($c.id) type=$($c.type) n=$($charts.Count)"

$ser = @($c.series) | Select-Object -First 1
$vals = @()
foreach ($v in @($ser.values)) {
    try { $vals += [double]$v } catch {}
}

$exported = Get-OutPath 'pptx-create-charts-export.xlsx'
if (Test-Path $exported) { Remove-Item $exported -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportChartData'
    path = $path.Replace('\', '/')
    id = $c.id
    output = $exported.Replace('\', '/')
} 'pptx-create-charts-export' | Out-Null

$treeE = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $exported.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:B4' }
} 'pptx-create-charts-embed-cells'
$b2 = @($treeE.sheets[0].cells) | Where-Object { $_.id -eq 'cell:Sheet1!B2' } | Select-Object -First 1
$b4 = @($treeE.sheets[0].cells) | Where-Object { $_.id -eq 'cell:Sheet1!B4' } | Select-Object -First 1
$embedOk = ($vals.Count -eq 3) -and ([int]$vals[0] -eq 1) -and ([int]$vals[2] -eq 3) -and ($null -ne $b2) -and ([int]$b2.value -eq 1) -and ([int]$b4.value -eq 3)
Add-Check 'pptx-C9' $embedOk "values=$($vals -join ',') b2=$($b2.value) b4=$($b4.value)"

$ppt = Ensure-MsPpt $path
Close-PptPresByPath $ppt $path
$pres = Open-PptPres $ppt $path
$cap = Get-PptCaption $pres
$s1 = Get-PptSlide $pres 1
$n = 0
$shapes = $s1.Shapes
try {
    for ($i = 1; $i -le $shapes.Count; $i++) {
        $sh = $shapes.Item($i)
        try {
            $has = $false
            try { $has = [bool]$sh.HasChart } catch {
                try { $has = ([int]$sh.Type -eq 3) } catch { $has = $false }
            }
            if ($has) { $n++ }
        }
        finally { Release-Com $sh }
    }
}
finally { Release-Com $shapes }
Release-Com $s1
Close-PptPres $pres
Add-Check 'pptx-C10' (($cap -notmatch 'Repair') -and ($n -ge 1)) "caption=$cap charts=$n"
