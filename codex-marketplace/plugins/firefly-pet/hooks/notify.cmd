@echo off
if not exist "%~dp0..\PetDesktop.exe" (
  echo {}
  exit /b 0
)
"%~dp0..\PetDesktop.exe" --codex-hook
exit /b 0
