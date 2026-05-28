!macro BEARPAD_REGISTER_OPEN_WITH EXTENSION
  WriteRegStr HKCU "Software\Classes\.${EXTENSION}\shell\Open with Bearpad" "" "Open with Bearpad"
  WriteRegStr HKCU "Software\Classes\.${EXTENSION}\shell\Open with Bearpad" "Icon" "$INSTDIR\Bearpad.exe"
  WriteRegStr HKCU "Software\Classes\.${EXTENSION}\shell\Open with Bearpad\command" "" "$\"$INSTDIR\Bearpad.exe$\" $\"%1$\""

  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXTENSION}\shell\Open with Bearpad" "" "Open with Bearpad"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXTENSION}\shell\Open with Bearpad" "Icon" "$INSTDIR\Bearpad.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXTENSION}\shell\Open with Bearpad\command" "" "$\"$INSTDIR\Bearpad.exe$\" $\"%1$\""

  WriteRegStr HKCU "Software\Classes\.${EXTENSION}\OpenWithList\Bearpad.exe" "" ""
!macroend

!macro BEARPAD_UNREGISTER_OPEN_WITH EXTENSION
  DeleteRegKey HKCU "Software\Classes\.${EXTENSION}\shell\Open with Bearpad"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.${EXTENSION}\shell\Open with Bearpad"
  DeleteRegKey HKCU "Software\Classes\.${EXTENSION}\OpenWithList\Bearpad.exe"
!macroend

!macro NSIS_HOOK_POST_INSTALL
  CreateShortcut "$DESKTOP\Bearpad.lnk" "$INSTDIR\Bearpad.exe" "" "$INSTDIR\Bearpad.exe" 0

  WriteRegStr HKCU "Software\Classes\Applications\Bearpad.exe\shell\open\command" "" "$\"$INSTDIR\Bearpad.exe$\" $\"%1$\""
  !insertmacro BEARPAD_REGISTER_OPEN_WITH "md"
  !insertmacro BEARPAD_REGISTER_OPEN_WITH "markdown"

  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend

!macro NSIS_HOOK_POST_UNINSTALL
  Delete "$DESKTOP\Bearpad.lnk"

  DeleteRegKey HKCU "Software\Classes\Applications\Bearpad.exe"
  !insertmacro BEARPAD_UNREGISTER_OPEN_WITH "md"
  !insertmacro BEARPAD_UNREGISTER_OPEN_WITH "markdown"

  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend
