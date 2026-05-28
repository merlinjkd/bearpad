$ErrorActionPreference = 'Stop'
$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
Uninstall-BinFile -Name 'bearpad'
Remove-Item "$( [Environment]::GetFolderPath('Desktop') )\Bearpad.lnk" -ErrorAction SilentlyContinue
Remove-Item "$( [Environment]::GetFolderPath('Programs') )\Bearpad.lnk" -ErrorAction SilentlyContinue
