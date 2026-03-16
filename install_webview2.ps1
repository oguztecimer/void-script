$out = Join-Path $env:TEMP 'MicrosoftEdgeWebview2Setup.exe'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
(New-Object Net.WebClient).DownloadFile('https://go.microsoft.com/fwlink/p/?LinkId=2124703', $out)
Start-Process -FilePath $out -ArgumentList '/silent','/install' -Verb RunAs -Wait
Write-Host "Done"
