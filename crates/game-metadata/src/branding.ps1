param([string]$ImagePath,[string]$OutputPath)
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false)
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null=[Windows.Storage.StorageFile,Windows.Storage,ContentType=WindowsRuntime]
$null=[Windows.Graphics.Imaging.BitmapDecoder,Windows.Foundation,ContentType=WindowsRuntime]
$null=[Windows.Media.Ocr.OcrEngine,Windows.Foundation,ContentType=WindowsRuntime]
$awaiter=[System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {$_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetGenericArguments().Count -eq 1 -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'} | Select-Object -First 1
function Await($operation,$resultType){$task=$awaiter.MakeGenericMethod($resultType).Invoke($null,@($operation));$task.GetAwaiter().GetResult()}
$stream=New-Object Windows.Storage.Streams.InMemoryRandomAccessStream
$writer=New-Object Windows.Storage.Streams.DataWriter($stream)
$writer.WriteBytes([System.IO.File]::ReadAllBytes($ImagePath))
$null=Await ($writer.StoreAsync()) ([uint32])
$null=$writer.DetachStream();$writer.Dispose();$stream.Seek(0)
$decoder=Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap=Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
$null=[Windows.Globalization.Language,Windows.Globalization,ContentType=WindowsRuntime]
$texts=@()
foreach($tag in @('zh-Hans-CN','en-US')){
 $language=New-Object Windows.Globalization.Language($tag)
 $engine=[Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage($language)
 if($null -eq $engine){continue}
 $result=Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
 $texts+=@{language=$tag;text=$result.Text.Substring(0,[Math]::Min(6000,$result.Text.Length))}
}
$bitmap.Dispose();$stream.Dispose()
$json=ConvertTo-Json -InputObject @($texts) -Depth 4 -Compress
[System.IO.File]::WriteAllText($OutputPath,$json,[System.Text.UTF8Encoding]::new($false))
