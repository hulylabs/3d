
$FontName = "Domine-Regular"
$OutputBaseName = "glyphs_atlas_ascii_domine"
$OutputFile = "$OutputBaseName.png"
$OutputFontFile = "$OutputBaseName.fnt"
$FontPath = "domine/static/$FontName.ttf"
$GeneratedFontFile = "$FontName.fnt"

$FontSize = 114
$TexturePadding = 2
$TextureEdgeBorder = 1
$SdfDistanceRange = 4

npx msdf-bmfont --reuse -o $OutputFile -s $FontSize -t msdf -p $TexturePadding -b $TextureEdgeBorder -r $SdfDistanceRange -v --pot $FontPath

if (Test-Path $GeneratedFontFile) {
    if (Test-Path $OutputFontFile) {
        Remove-Item -Path $OutputFontFile -Force
        Write-Host "Removed existing $OutputFontFile"
    }
    Rename-Item -Path $GeneratedFontFile -NewName $OutputFontFile
    Write-Host "Renamed $GeneratedFontFile to $OutputFontFile"
} else {
    Write-Warning "Generated font file $GeneratedFontFile not found"
}
