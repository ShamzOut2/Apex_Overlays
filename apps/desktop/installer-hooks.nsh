; Installer hooks (bundle.windows.nsis.installerHooks in tauri.conf.json).

!macro NSIS_HOOK_POSTINSTALL
  ; Windows keeps showing the icon it cached for the app (pinned taskbar and Start menu
  ; shortcuts too) after an update changes it. Tell Explorer icons have changed
  ; (SHCNE_ASSOCCHANGED) and refresh its icon cache, so the new icon shows.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0x1000, p 0, p 0)'
  Push $0
  ; A 32-bit installer on 64-bit Windows reaches the 64-bit tool through Sysnative.
  IfFileExists "$WINDIR\Sysnative\ie4uinit.exe" 0 apex_icons_sys32
    nsExec::Exec '"$WINDIR\Sysnative\ie4uinit.exe" -show'
    Goto apex_icons_done
  apex_icons_sys32:
    nsExec::Exec '"$SYSDIR\ie4uinit.exe" -show'
  apex_icons_done:
  Pop $0
  Pop $0
!macroend
