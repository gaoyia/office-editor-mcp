# 阶段二 6 到 10：库写出的页背景、渐变、图片填充、图层和负坐标，PowerPoint 16.0 打开无 Repair。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$png = Ensure-OoxpPngA
$seed = Get-OutPath 'pptx-paint-seed.pptx'
$painted = Get-OutPath 'pptx-paint.pptx'
$created = Get-OutPath 'pptx-paint-create.pptx'

Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $seed
    presentation  = @{
        slides = @(
            @{ shapes = @(@{ text = '底' }, @{ text = '上' }) }
        )
    }
} 'pptx-paint-seed'

Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'apply'
    path          = $seed
    output        = $painted
    ops           = @(
        @{
            op    = 'setPageBackground'
            id    = 'slide:1'
            color = 'FF5C2B'
            alpha = 65000
        }
        @{
            op   = 'setShapeFill'
            id   = 'slide:1/shape:2'
            fill = @{
                type  = 'grad'
                alpha = 50000
                stops = @(
                    @{ pos = 0; color = 'FF0000' }
                    @{ pos = 100000; color = '0000FF' }
                )
            }
        }
        @{
            op    = 'setShapeFill'
            id    = 'slide:1/shape:3'
            fill  = @{ type = 'blip'; file = $png }
        }
        @{
            op    = 'insertShape'
            id    = 'slide:1'
            prst  = 'ellipse'
            x     = -457200
            y     = -228600
            index = 0
            fill  = '2B5CFF'
            line  = @{ color = '334455'; width = 25400 }
        }
        @{
            op    = 'insertPicture'
            id    = 'slide:1'
            file  = $png
            index = 0
        }
    )
} 'pptx-paint-apply'

Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $created
    presentation  = @{
        slides = @(
            @{
                shapes = @(
                    @{
                        text = '圆'
                        prst = 'ellipse'
                        x    = -457200
                        y    = -228600
                        fill = @{ type = 'solid'; color = 'FF5C2B'; alpha = 65000 }
                    }
                )
            }
        )
    }
} 'pptx-paint-create'

$ppt = Ensure-MsPpt $painted
$pres = $null
$slide = $null
$ellipse = $null
$pic = $null
try {
    $pres = Open-PptPres $ppt $painted
    $caption = Get-PptCaption $pres
    $slide = Get-PptSlide $pres 1
    $follow = [int]$slide.FollowMasterBackground
    $ellipse = Get-PptShape $slide '形状 4'
    $left = [double]$ellipse.Left
    $pic = Get-PptShape $slide '图片 5'
    $picZ = [int]$pic.ZOrderPosition
    $ellZ = [int]$ellipse.ZOrderPosition
    $ok = ($caption -notmatch 'Repair') -and ($follow -eq 0) -and ($left -lt 0) -and ($picZ -lt $ellZ)
    Add-Check 'pptx-paint-apply' $ok "cap=$caption follow=$follow left=$left picZ=$picZ ellZ=$ellZ"
}
finally {
    Release-Com $pic
    Release-Com $ellipse
    Release-Com $slide
    Close-PptPres $pres
}

$pres2 = $null
$slide2 = $null
$shape2 = $null
try {
    $pres2 = Open-PptPres $ppt $created
    $caption2 = Get-PptCaption $pres2
    $slide2 = Get-PptSlide $pres2 1
    $shape2 = Get-PptShape $slide2 '形状 2'
    $left2 = [double]$shape2.Left
    $ok2 = ($caption2 -notmatch 'Repair') -and ($left2 -lt 0)
    Add-Check 'pptx-paint-create' $ok2 "cap=$caption2 left=$left2"
}
finally {
    Release-Com $shape2
    Release-Com $slide2
    Close-PptPres $pres2
}
