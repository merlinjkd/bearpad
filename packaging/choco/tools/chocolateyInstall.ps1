$ErrorActionPreference = 'Stop'
$toolsDir   = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
Install-BinFile -Name 'bearpad' -Path "$toolsDir\Bearpad.exe"
Install-ChocolateyShortcut -ShortcutFilePath "$( [Environment]::GetFolderPath('Desktop') )\Bearpad.lnk" -TargetPath "$toolsDir\Bearpad.exe"
Install-ChocolateyShortcut -ShortcutFilePath "$( [Environment]::GetFolderPath('Programs') )\Bearpad.lnk" -TargetPath "$toolsDir\Bearpad.exe"
