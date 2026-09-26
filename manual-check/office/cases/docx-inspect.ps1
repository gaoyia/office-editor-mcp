# docx inspect：docs/checklist-docx.md 的 I1–I5。
# 只调 ooxp，不开 Word。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$path = (Ensure-DocxCreate).Replace('\', '/')

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path; format = 'docx'
    scope = @{ from = 1; maxParas = 200 }
} 'docx-inspect'

Add-Check 'docx-I1' (($tree.id -eq 'docx:document') -and ($tree.paraCount -eq 2)) "id=$($tree.id) count=$($tree.paraCount)"

$ol = @($tree.outline)
$olOk = ($ol.Count -ge 1) -and ($ol[0].level -eq 1) -and ($ol[0].text -eq $S.title)
Add-Check 'docx-I2' $olOk ($ol | ConvertTo-Json -Compress)

$paras = @($tree.paragraphs)
# 两段都该拿到 w14:paraId，而不是退化成 para:index:n。
$idOk = ($paras.Count -eq 2) -and $paras[0].stable -and $paras[1].stable -and
        ($paras[0].id -like 'para:*') -and ($paras[0].id -notlike 'para:index:*')
Add-Check 'docx-I3' $idOk "$($paras[0].id),$($paras[1].id)"
Add-Check 'docx-I4' ($paras[1].text -eq $S.hello) $paras[1].text

# 不给 scope 时不许返回段落正文，但 outline 和 paraCount 要照常给。
$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path; format = 'docx'
} 'docx-inspect-outline'

$noParas = -not ($tree2.PSObject.Properties.Name -contains 'paragraphs')
$hasOl = $tree2.PSObject.Properties.Name -contains 'outline'
$hasHf = ($tree2.PSObject.Properties.Name -contains 'headers') -and ($tree2.PSObject.Properties.Name -contains 'footers')
Add-Check 'docx-I5' ($noParas -and $hasOl -and $hasHf -and ($tree2.paraCount -eq 2)) "noParas=$noParas outline=$hasOl hf=$hasHf count=$($tree2.paraCount)"
