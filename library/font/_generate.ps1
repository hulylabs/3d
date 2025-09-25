
$FontName = "Domine-Regular"
$OutputBaseName = "glyphs_atlas"
$OutputFile = "$OutputBaseName.png"
$OutputFontFile = "$OutputBaseName.fnt"
$FontPath = "domine/static/$FontName.ttf"
$GeneratedFontFile = "$FontName.fnt"

$FontSize = 114
$TexturePadding = 2
$TextureEdgeBorder = 1
$SdfDistanceRange = 5

npx msdf-bmfont --reuse -o $OutputFile -s $FontSize -t msdf -p $TexturePadding -b $TextureEdgeBorder -r $SdfDistanceRange -v --pot $FontPath

if (Test-Path $GeneratedFontFile) {
    Rename-Item -Path $GeneratedFontFile -NewName $OutputFontFile
    Write-Host "Renamed $GeneratedFontFile to $OutputFontFile"
} else {
    Write-Warning "Generated font file $GeneratedFontFile not found"
}
