# Rebuild the native icons from the same audio-bars/play geometry as public/icon.svg.
Add-Type -AssemblyName System.Drawing
$taskRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$taskIconDir = Join-Path $taskRoot 'src-tauri/icons'
$taskPngs = @()
foreach ($taskSize in @(32, 128, 256, 512)) {
    $taskBitmap = [Drawing.Bitmap]::new($taskSize, $taskSize)
    $taskGraphics = [Drawing.Graphics]::FromImage($taskBitmap)
    $taskGraphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $taskGraphics.ScaleTransform($taskSize / 512.0, $taskSize / 512.0)
    $taskPath = [Drawing.Drawing2D.GraphicsPath]::new()
    $taskPath.AddArc(16, 16, 224, 224, 180, 90)
    $taskPath.AddArc(272, 16, 224, 224, 270, 90)
    $taskPath.AddArc(272, 272, 224, 224, 0, 90)
    $taskPath.AddArc(16, 272, 224, 224, 90, 90)
    $taskPath.CloseFigure()
    $taskTile = [Drawing.Drawing2D.LinearGradientBrush]::new([Drawing.Point]::new(16,16), [Drawing.Point]::new(496,496), [Drawing.ColorTranslator]::FromHtml('#353747'), [Drawing.ColorTranslator]::FromHtml('#171b28'))
    $taskGraphics.FillPath($taskTile, $taskPath)
    $taskBorder = [Drawing.Pen]::new([Drawing.Color]::FromArgb(40,255,255,255), 2)
    $taskGraphics.DrawPath($taskBorder, $taskPath)
    $taskSound = [Drawing.Drawing2D.LinearGradientBrush]::new([Drawing.Point]::new(135,180), [Drawing.Point]::new(407,332), [Drawing.ColorTranslator]::FromHtml('#eee8ff'), [Drawing.ColorTranslator]::FromHtml('#b5a5ec'))
    $taskPen = [Drawing.Pen]::new($taskSound, 30)
    $taskPen.StartCap = [Drawing.Drawing2D.LineCap]::Round
    $taskPen.EndCap = [Drawing.Drawing2D.LineCap]::Round
    $taskGraphics.DrawLine($taskPen, 135,226,135,286)
    $taskGraphics.DrawLine($taskPen, 201,180,201,332)
    $taskGraphics.DrawLine($taskPen, 267,211,267,301)
    $taskTriangle = [Drawing.Drawing2D.GraphicsPath]::new()
    $taskTriangle.AddBezier(329,208,329,195,332,197,340,202)
    $taskTriangle.AddLine(340,202,407,247)
    $taskTriangle.AddBezier(407,247,419,256,419,256,407,265)
    $taskTriangle.AddLine(407,265,340,310)
    $taskTriangle.AddBezier(340,310,329,317,329,317,329,304)
    $taskTriangle.CloseFigure()
    $taskGraphics.FillPath($taskSound,$taskTriangle)
    $taskStream = [IO.MemoryStream]::new()
    $taskBitmap.Save($taskStream,[Drawing.Imaging.ImageFormat]::Png)
    $taskPngs += ,@($taskSize,$taskStream.ToArray())
    if ($taskSize -eq 32 -or $taskSize -eq 128) { [IO.File]::WriteAllBytes((Join-Path $taskIconDir "${taskSize}x${taskSize}.png"),$taskStream.ToArray()) }
    if ($taskSize -eq 512) { [IO.File]::WriteAllBytes((Join-Path $taskIconDir 'icon.png'),$taskStream.ToArray()) }
    $taskStream.Dispose(); $taskTriangle.Dispose(); $taskPen.Dispose(); $taskSound.Dispose(); $taskBorder.Dispose(); $taskTile.Dispose(); $taskPath.Dispose(); $taskGraphics.Dispose(); $taskBitmap.Dispose()
}
$taskIco = [IO.MemoryStream]::new()
$taskWriter = [IO.BinaryWriter]::new($taskIco)
$taskEntries = @($taskPngs | Where-Object { $_[0] -le 256 })
$taskWriter.Write([uint16]0); $taskWriter.Write([uint16]1); $taskWriter.Write([uint16]$taskEntries.Count)
$taskOffset = 6 + 16 * $taskEntries.Count
foreach ($taskEntry in $taskEntries) {
    $taskDimension = if ($taskEntry[0] -eq 256) { 0 } else { $taskEntry[0] }
    $taskWriter.Write([byte]$taskDimension); $taskWriter.Write([byte]$taskDimension)
    $taskWriter.Write([byte]0); $taskWriter.Write([byte]0); $taskWriter.Write([uint16]1); $taskWriter.Write([uint16]32)
    $taskWriter.Write([uint32]$taskEntry[1].Length); $taskWriter.Write([uint32]$taskOffset)
    $taskOffset += $taskEntry[1].Length
}
foreach ($taskEntry in $taskEntries) { $taskWriter.Write([byte[]]$taskEntry[1]) }
[IO.File]::WriteAllBytes((Join-Path $taskIconDir 'icon.ico'), $taskIco.ToArray())
$taskWriter.Dispose(); $taskIco.Dispose()
Write-Output 'Generated PNG and multi-resolution ICO icons.'
